use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::cli::GetArgs;
use crate::dom::Document;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct CrawlArgs {
    /// Starting URL (or use current page if omitted)
    pub url: Option<String>,

    /// Max depth to crawl (0 = just list links, 1 = follow one level, etc.)
    #[arg(long, short, default_value = "1")]
    pub depth: usize,

    /// Max total pages to fetch
    #[arg(long, default_value = "20")]
    pub max_pages: usize,

    /// Only follow links matching this substring
    #[arg(long)]
    pub filter: Option<String>,

    /// Only follow links matching this CSS selector
    #[arg(long, default_value = "a[href]")]
    pub selector: String,

    /// Allow following links to other domains
    #[arg(long)]
    pub cross_domain: bool,

    /// Extract text content from each page (adds 'text' field)
    #[arg(long)]
    pub extract_text: bool,

    /// CSS selector for text extraction (used with --extract-text)
    #[arg(long, default_value = "body")]
    pub text_selector: String,

    /// Max text length per page
    #[arg(long, default_value = "500")]
    pub text_max_length: usize,
}

#[derive(Serialize)]
struct CrawlResult {
    pages_fetched: usize,
    max_depth_reached: usize,
    tree: Vec<CrawlNode>,
}

#[derive(Serialize, Clone)]
struct CrawlNode {
    url: String,
    title: Option<String>,
    depth: usize,
    status: Option<u16>,
    link_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<ChildLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Clone)]
struct ChildLink {
    url: String,
    text: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    visited: bool,
}

pub async fn execute(args: CrawlArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    // Determine starting URL
    let start_url = if let Some(ref url) = args.url {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.clone()
        } else if let Some(ref current) = session.state.current_url {
            url::Url::parse(current)?.join(url)?.to_string()
        } else {
            format!("https://{}", url)
        }
    } else if let Some(ref current) = session.state.current_url {
        current.clone()
    } else {
        return Err(anyhow::anyhow!("No URL specified and no current page. Run `clibrowser get <url>` first or pass a URL."));
    };

    let start_domain = url::Url::parse(&start_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let mut visited: HashSet<String> = HashSet::new();
    let mut nodes: Vec<CrawlNode> = Vec::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut max_depth_reached = 0;

    queue.push_back((start_url.clone(), 0));

    while let Some((url, depth)) = queue.pop_front() {
        // Normalize URL (remove fragment)
        let normalized = normalize_url(&url);
        if visited.contains(&normalized) {
            continue;
        }
        if nodes.len() >= args.max_pages {
            break;
        }

        visited.insert(normalized.clone());

        if depth > max_depth_reached {
            max_depth_reached = depth;
        }

        // Print progress in human mode
        if !out.json && !out.quiet {
            let indent = "  ".repeat(depth);
            eprint!("{}[depth={}] Fetching: {} ...", indent, depth, truncate_str(&normalized, 60));
        }

        // Fetch the page
        let get_args = GetArgs {
            url: normalized.clone(),
            method: "GET".to_string(),
            headers: vec![],
            data: None,
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: 15,
            user_agent: None,
            stealth,
        };

        match crate::http::fetch(&get_args, session).await {
            Ok(response) => {
                if !out.json && !out.quiet {
                    eprintln!(" {}", response.status);
                }

                let doc = Document::parse(&response.body);

                // Extract title
                let title = doc
                    .select("title")
                    .ok()
                    .and_then(|els| els.first().map(|e| e.text.clone()));

                // Extract text if requested
                let text = if args.extract_text {
                    let mut t = doc
                        .extract_text(&args.text_selector)
                        .unwrap_or_default();
                    // Normalize whitespace
                    t = t
                        .lines()
                        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    if t.len() > args.text_max_length {
                        t.truncate(args.text_max_length);
                        t.push_str("...");
                    }
                    Some(t)
                } else {
                    None
                };

                // Extract links
                let base_url = url::Url::parse(&response.url).ok();
                let links = doc
                    .extract_links(&args.selector, base_url.as_ref())
                    .unwrap_or_default();

                let mut children: Vec<ChildLink> = Vec::new();

                for link in &links {
                    let href = normalize_url(&link.href);

                    // Skip non-http links
                    if !href.starts_with("http://") && !href.starts_with("https://") {
                        continue;
                    }

                    // Apply domain filter
                    if !args.cross_domain {
                        let link_domain = url::Url::parse(&href)
                            .ok()
                            .and_then(|u| u.host_str().map(|s| s.to_string()))
                            .unwrap_or_default();
                        if link_domain != start_domain {
                            continue;
                        }
                    }

                    // Apply substring filter
                    if let Some(ref filter) = args.filter {
                        if !href.contains(filter) && !link.text.contains(filter) {
                            continue;
                        }
                    }

                    let already_visited = visited.contains(&href);

                    children.push(ChildLink {
                        url: href.clone(),
                        text: link.text.clone(),
                        visited: already_visited,
                    });

                    // Queue for crawling if within depth
                    if !already_visited && depth < args.depth {
                        queue.push_back((href, depth + 1));
                    }
                }

                // Deduplicate children by URL
                let mut seen_urls: HashSet<String> = HashSet::new();
                children.retain(|c| seen_urls.insert(c.url.clone()));

                nodes.push(CrawlNode {
                    url: response.url,
                    title,
                    depth,
                    status: Some(response.status),
                    link_count: children.len(),
                    text,
                    children,
                    error: None,
                });
            }
            Err(e) => {
                if !out.json && !out.quiet {
                    eprintln!(" ERROR: {}", e);
                }
                nodes.push(CrawlNode {
                    url: normalized,
                    title: None,
                    depth,
                    status: None,
                    link_count: 0,
                    text: None,
                    children: vec![],
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let result = CrawlResult {
        pages_fetched: nodes.len(),
        max_depth_reached,
        tree: nodes,
    };

    if out.json {
        out.print_json(&result);
    } else {
        println!();
        println!("Crawl complete: {} pages, max depth {}", result.pages_fetched, result.max_depth_reached);
        println!();

        for node in &result.tree {
            let indent = "  ".repeat(node.depth);
            let title = node.title.as_deref().unwrap_or("(no title)");
            let status = node
                .status
                .map(|s| s.to_string())
                .unwrap_or_else(|| "ERR".to_string());

            println!(
                "{}[{}] {} - {} ({} links)",
                indent, status, truncate_str(&node.url, 60), title, node.link_count
            );

            if let Some(ref text) = node.text {
                for line in text.lines().take(3) {
                    println!("{}  > {}", indent, truncate_str(line, 70));
                }
            }

            if let Some(ref err) = node.error {
                println!("{}  ERROR: {}", indent, err);
            }
        }
    }

    Ok(0)
}

fn normalize_url(url: &str) -> String {
    // Remove fragment
    if let Some(pos) = url.find('#') {
        url[..pos].to_string()
    } else {
        url.to_string()
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
