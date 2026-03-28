use anyhow::Result;
use serde::Serialize;
use std::io::BufRead;

use crate::cli::GetArgs;
use crate::dom::Document;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct PipeArgs {
    /// Extract text from each page (CSS selector)
    #[arg(long, default_value = "body")]
    pub selector: String,

    /// Max text length per page
    #[arg(long, default_value = "500")]
    pub max_text: usize,

    /// Also extract page title
    #[arg(long)]
    pub title: bool,

    /// Also extract links
    #[arg(long)]
    pub links: bool,

    /// Strip whitespace aggressively
    #[arg(long)]
    pub strip: bool,

    /// Continue on errors (don't stop on failed URLs)
    #[arg(long)]
    pub continue_on_error: bool,

    /// Request timeout per URL
    #[arg(long, default_value = "15")]
    pub timeout: u64,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(Serialize)]
struct PipeResult {
    total: usize,
    succeeded: usize,
    failed: usize,
    pages: Vec<PageResult>,
}

#[derive(Serialize)]
struct PageResult {
    url: String,
    status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    links: Option<Vec<LinkItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct LinkItem {
    href: String,
    text: String,
}

pub async fn execute(mut args: PipeArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;

    // Read URLs from stdin
    let stdin = std::io::stdin();
    let urls: Vec<String> = stdin
        .lock()
        .lines()
        .filter_map(|l| l.ok())
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if urls.is_empty() {
        return Err(anyhow::anyhow!("No URLs provided on stdin. Usage: cat urls.txt | clibrowser pipe"));
    }

    let total = urls.len();
    let mut succeeded = 0;
    let mut failed = 0;
    let mut pages: Vec<PageResult> = Vec::new();

    for (i, url) in urls.iter().enumerate() {
        if !out.quiet && !out.json {
            eprint!("[{}/{}] {} ... ", i + 1, total, truncate(url, 60));
        }

        let get_args = GetArgs {
            url: url.clone(),
            method: "GET".to_string(),
            headers: vec![],
            data: None,
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: args.timeout,
            user_agent: None,
            stealth: args.stealth,
        };

        match crate::http::fetch(&get_args, session).await {
            Ok(response) => {
                if !out.quiet && !out.json {
                    eprintln!("{}", response.status);
                }

                let doc = Document::parse(&response.body);

                let title = if args.title {
                    doc.select("title")
                        .ok()
                        .and_then(|els| els.first().map(|e| e.text.clone()))
                } else {
                    None
                };

                let text = {
                    let mut t = doc.extract_text(&args.selector).unwrap_or_default();
                    if args.strip {
                        t = t
                            .lines()
                            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
                            .filter(|l| !l.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                    if t.len() > args.max_text {
                        t.truncate(args.max_text);
                        t.push_str("...");
                    }
                    Some(t)
                };

                let link_items = if args.links {
                    let base_url = url::Url::parse(&response.url).ok();
                    let extracted = doc.extract_links("a[href]", base_url.as_ref()).unwrap_or_default();
                    Some(
                        extracted
                            .into_iter()
                            .map(|l| LinkItem {
                                href: l.href,
                                text: l.text,
                            })
                            .collect(),
                    )
                } else {
                    None
                };

                pages.push(PageResult {
                    url: response.url,
                    status: Some(response.status),
                    title,
                    text,
                    links: link_items,
                    error: None,
                });
                succeeded += 1;
            }
            Err(e) => {
                if !out.quiet && !out.json {
                    eprintln!("ERROR: {}", e);
                }
                pages.push(PageResult {
                    url: url.clone(),
                    status: None,
                    title: None,
                    text: None,
                    links: None,
                    error: Some(e.to_string()),
                });
                failed += 1;

                if !args.continue_on_error {
                    break;
                }
            }
        }
    }

    if out.json {
        out.print_json(&PipeResult {
            total,
            succeeded,
            failed,
            pages,
        });
    } else {
        println!();
        println!("Pipe complete: {}/{} succeeded, {} failed", succeeded, total, failed);
        println!();
        for page in &pages {
            let status = page.status.map(|s| s.to_string()).unwrap_or_else(|| "ERR".to_string());
            println!("[{}] {}", status, page.url);
            if let Some(ref title) = page.title {
                println!("  Title: {}", title);
            }
            if let Some(ref text) = page.text {
                for line in text.lines().take(3) {
                    println!("  > {}", truncate(line, 80));
                }
            }
            if let Some(ref err) = page.error {
                println!("  Error: {}", err);
            }
            println!();
        }
    }

    if failed > 0 { Ok(1) } else { Ok(0) }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
