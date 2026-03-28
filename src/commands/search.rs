use anyhow::Result;
use serde::Serialize;

use crate::cli::GetArgs;
use crate::dom::Document;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct SearchArgs {
    /// Search query
    pub query: Vec<String>,

    /// Max results to return
    #[arg(long, short = 'n', default_value = "10")]
    pub max_results: usize,

    /// Search engine to use
    #[arg(long, default_value = "duckduckgo")]
    pub engine: String,

    /// Open the first result automatically
    #[arg(long)]
    pub lucky: bool,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(Serialize)]
struct SearchResult {
    query: String,
    engine: String,
    count: usize,
    results: Vec<SearchItem>,
}

#[derive(Serialize, Clone)]
struct SearchItem {
    index: usize,
    title: String,
    url: String,
    snippet: String,
}

pub async fn execute(mut args: SearchArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;
    let query = args.query.join(" ");

    if query.is_empty() {
        return Err(anyhow::anyhow!("Search query cannot be empty"));
    }

    let results = match args.engine.as_str() {
        "duckduckgo" | "ddg" => search_duckduckgo(&query, &args, session).await?,
        "google" => search_google(&query, &args, session).await?,
        other => return Err(anyhow::anyhow!("Unknown search engine: {}. Use 'duckduckgo' or 'google'.", other)),
    };

    let items: Vec<SearchItem> = results.into_iter().take(args.max_results).collect();

    // If --lucky, navigate to the first result
    if args.lucky && !items.is_empty() {
        let get_args = GetArgs {
            url: items[0].url.clone(),
            method: "GET".to_string(),
            headers: vec![],
            data: None,
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: 30,
            user_agent: None,
            stealth: args.stealth,
        };
        if !out.quiet && !out.json {
            out.print_human(&format!("Navigating to: {} ({})", items[0].title, items[0].url));
        }
        return super::navigate::execute(get_args, session, out).await;
    }

    if out.json {
        out.print_json(&SearchResult {
            query: query.clone(),
            engine: args.engine.clone(),
            count: items.len(),
            results: items,
        });
    } else {
        out.print_human(&format!("Search: \"{}\" ({}, {} results)\n", query, args.engine, items.len()));
        for item in &items {
            out.print_human(&format!("[{}] {}", item.index, item.title));
            out.print_human(&format!("    {}", item.url));
            if !item.snippet.is_empty() {
                out.print_human(&format!("    {}", item.snippet));
            }
            out.print_human("");
        }
    }

    Ok(0)
}

async fn search_duckduckgo(query: &str, args: &SearchArgs, session: &mut Session) -> Result<Vec<SearchItem>> {
    let encoded = urlencoding_encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

    let get_args = GetArgs {
        url,
        method: "GET".to_string(),
        headers: vec![],
        data: None,
        data_json: None,
        no_follow: false,
        max_redirects: 10,
        timeout: 15,
        user_agent: None,
        stealth: args.stealth,
    };

    let response = crate::http::fetch(&get_args, session).await?;
    let doc = Document::parse(&response.body);

    // DuckDuckGo HTML results are in .result elements
    let result_elements = doc.select(".result")?;

    let mut items = Vec::new();
    for (i, el) in result_elements.iter().enumerate() {
        if i >= args.max_results {
            break;
        }

        // Parse each result - DDG structure:
        // .result__a = title + link
        // .result__snippet = description
        let title = el.text.lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        // Extract URL from the result HTML
        let url = extract_ddg_url(&el.html);
        let snippet = extract_ddg_snippet(&el.html);

        if !url.is_empty() && !title.is_empty() {
            items.push(SearchItem {
                index: i,
                title: clean_text(&title),
                url,
                snippet: clean_text(&snippet),
            });
        }
    }

    // If the .result selector didn't work, try alternative parsing
    if items.is_empty() {
        items = parse_ddg_fallback(&response.body, args.max_results);
    }

    Ok(items)
}

fn extract_ddg_url(html: &str) -> String {
    // Look for href in result__a or result-link
    if let Some(start) = html.find("href=\"") {
        let rest = &html[start + 6..];
        if let Some(end) = rest.find('"') {
            let raw_url = &rest[..end];
            // DDG wraps URLs in a redirect: //duckduckgo.com/l/?uddg=ENCODED_URL
            if raw_url.contains("uddg=") {
                if let Some(uddg_start) = raw_url.find("uddg=") {
                    let encoded = &raw_url[uddg_start + 5..];
                    let encoded = encoded.split('&').next().unwrap_or(encoded);
                    return urldecode(encoded);
                }
            }
            if raw_url.starts_with("http") {
                return raw_url.to_string();
            }
        }
    }
    String::new()
}

fn extract_ddg_snippet(html: &str) -> String {
    // Find result__snippet class
    if let Some(start) = html.find("result__snippet") {
        let rest = &html[start..];
        if let Some(tag_start) = rest.find('>') {
            let content = &rest[tag_start + 1..];
            if let Some(tag_end) = content.find("</") {
                let snippet = &content[..tag_end];
                return strip_html_tags(snippet);
            }
        }
    }
    String::new()
}

fn parse_ddg_fallback(html: &str, max: usize) -> Vec<SearchItem> {
    let doc = Document::parse(html);
    let mut items = Vec::new();

    // Try to find links that look like search results
    if let Ok(links) = doc.select("a.result__a") {
        for (i, link) in links.iter().enumerate() {
            if i >= max { break; }
            let title = link.text.clone();
            let url = link.attributes.get("href").cloned().unwrap_or_default();
            let resolved_url = if url.contains("uddg=") {
                if let Some(start) = url.find("uddg=") {
                    let encoded = &url[start + 5..];
                    let encoded = encoded.split('&').next().unwrap_or(encoded);
                    urldecode(encoded)
                } else {
                    url
                }
            } else {
                url
            };

            if !resolved_url.is_empty() {
                items.push(SearchItem {
                    index: i,
                    title: clean_text(&title),
                    url: resolved_url,
                    snippet: String::new(),
                });
            }
        }
    }

    items
}

async fn search_google(query: &str, args: &SearchArgs, session: &mut Session) -> Result<Vec<SearchItem>> {
    let encoded = urlencoding_encode(query);
    let url = format!("https://www.google.com/search?q={}&num={}", encoded, args.max_results);

    let get_args = GetArgs {
        url,
        method: "GET".to_string(),
        headers: vec![],
        data: None,
        data_json: None,
        no_follow: false,
        max_redirects: 10,
        timeout: 15,
        user_agent: None,
        stealth: true, // Always stealth for Google
    };

    let response = crate::http::fetch(&get_args, session).await?;
    let doc = Document::parse(&response.body);

    let mut items = Vec::new();

    // Google results: look for links within the main content
    if let Ok(links) = doc.extract_links("a[href]", None) {
        for link in &links {
            if items.len() >= args.max_results {
                break;
            }

            // Filter Google result links (they start with /url?q=)
            let url = if link.href.contains("/url?q=") {
                if let Some(start) = link.href.find("/url?q=") {
                    let encoded = &link.href[start + 7..];
                    let encoded = encoded.split('&').next().unwrap_or(encoded);
                    urldecode(encoded)
                } else {
                    continue;
                }
            } else if link.href.starts_with("http") && !link.href.contains("google.com") {
                link.href.clone()
            } else {
                continue;
            };

            // Skip Google internal links
            if url.contains("google.com") || url.contains("accounts.google") {
                continue;
            }

            let title = if link.text.is_empty() {
                url.clone()
            } else {
                link.text.clone()
            };

            items.push(SearchItem {
                index: items.len(),
                title: clean_text(&title),
                url,
                snippet: String::new(),
            });
        }
    }

    Ok(items)
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

fn urldecode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

fn clean_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}
