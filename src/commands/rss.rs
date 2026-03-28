use anyhow::Result;
use serde::Serialize;

use crate::cli::GetArgs;
use crate::dom::Document;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct RssArgs {
    /// RSS/Atom feed URL
    pub url: String,

    /// Max items to return
    #[arg(long, short = 'n', default_value = "20")]
    pub max_items: usize,

    /// Only items newer than this (e.g. "2026-03-20", "7d", "24h")
    #[arg(long)]
    pub since: Option<String>,

    /// Filter items by keyword in title or description
    #[arg(long)]
    pub filter: Option<String>,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(Serialize)]
struct RssResult {
    feed_title: Option<String>,
    feed_url: String,
    count: usize,
    items: Vec<RssItem>,
}

#[derive(Serialize, Clone)]
struct RssItem {
    index: usize,
    title: String,
    url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
}

pub async fn execute(mut args: RssArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;

    let get_args = GetArgs {
        url: args.url.clone(),
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

    // Detect feed type and parse
    let items = if response.body.contains("<rss") || response.body.contains("<channel") {
        parse_rss(&response.body)?
    } else if response.body.contains("<feed") {
        parse_atom(&response.body)?
    } else {
        // Try RSS autodiscovery from HTML page
        let doc = Document::parse(&response.body);
        if let Ok(links) = doc.select("link[type='application/rss+xml'], link[type='application/atom+xml']") {
            if let Some(link) = links.first() {
                if let Some(href) = link.attributes.get("href") {
                    // Fetch the discovered feed
                    let feed_url = if href.starts_with("http") {
                        href.clone()
                    } else if let Some(ref current) = session.state.current_url {
                        url::Url::parse(current)?.join(href)?.to_string()
                    } else {
                        href.clone()
                    };

                    let feed_args = GetArgs {
                        url: feed_url,
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
                    let feed_response = crate::http::fetch(&feed_args, session).await?;

                    if feed_response.body.contains("<rss") || feed_response.body.contains("<channel") {
                        parse_rss(&feed_response.body)?
                    } else {
                        parse_atom(&feed_response.body)?
                    }
                } else {
                    return Err(anyhow::anyhow!("No RSS/Atom feed found at this URL"));
                }
            } else {
                return Err(anyhow::anyhow!("No RSS/Atom feed found at this URL. Try providing a direct feed URL."));
            }
        } else {
            return Err(anyhow::anyhow!("Content is not an RSS/Atom feed and no feed link found in the page."));
        }
    };

    // Extract feed title
    let feed_title = extract_feed_title(&response.body);

    // Apply filters
    let mut filtered: Vec<RssItem> = items;

    if let Some(ref keyword) = args.filter {
        let kw = keyword.to_lowercase();
        filtered.retain(|item| {
            item.title.to_lowercase().contains(&kw)
                || item.description.to_lowercase().contains(&kw)
        });
    }

    if let Some(ref since) = args.since {
        if let Some(cutoff) = parse_since(since) {
            filtered.retain(|item| {
                if let Some(ref pub_date) = item.published {
                    pub_date >= &cutoff
                } else {
                    true // Keep items without dates
                }
            });
        }
    }

    // Apply max items
    filtered.truncate(args.max_items);

    // Re-index
    for (i, item) in filtered.iter_mut().enumerate() {
        item.index = i;
    }

    if out.json {
        out.print_json(&RssResult {
            feed_title: feed_title.clone(),
            feed_url: args.url.clone(),
            count: filtered.len(),
            items: filtered,
        });
    } else {
        let title = feed_title.as_deref().unwrap_or("(unknown feed)");
        out.print_human(&format!("Feed: {} ({} items)\n", title, filtered.len()));
        for item in &filtered {
            let date = item.published.as_deref().unwrap_or("");
            out.print_human(&format!("[{}] {} {}", item.index, item.title, if date.is_empty() { String::new() } else { format!("({})", &date[..10.min(date.len())]) }));
            out.print_human(&format!("    {}", item.url));
            if !item.description.is_empty() {
                let desc = if item.description.len() > 120 {
                    format!("{}...", &item.description[..117])
                } else {
                    item.description.clone()
                };
                out.print_human(&format!("    {}", desc));
            }
            out.print_human("");
        }
    }

    Ok(0)
}

fn parse_rss(xml: &str) -> Result<Vec<RssItem>> {
    let mut items = Vec::new();
    let mut pos = 0;

    while let Some(item_start) = xml[pos..].find("<item") {
        let abs_start = pos + item_start;
        if let Some(item_end) = xml[abs_start..].find("</item>") {
            let item_xml = &xml[abs_start..abs_start + item_end + 7];

            let title = extract_xml_tag(item_xml, "title");
            let link = extract_xml_tag(item_xml, "link");
            let description = extract_xml_tag(item_xml, "description");
            let pub_date = extract_xml_tag(item_xml, "pubDate")
                .or_else(|| extract_xml_tag(item_xml, "dc:date"));
            let author = extract_xml_tag(item_xml, "author")
                .or_else(|| extract_xml_tag(item_xml, "dc:creator"));

            items.push(RssItem {
                index: items.len(),
                title: strip_cdata(&title.unwrap_or_default()),
                url: link.unwrap_or_default(),
                description: clean_description(&description.unwrap_or_default()),
                published: pub_date,
                author,
            });

            pos = abs_start + item_end + 7;
        } else {
            break;
        }
    }

    Ok(items)
}

fn parse_atom(xml: &str) -> Result<Vec<RssItem>> {
    let mut items = Vec::new();
    let mut pos = 0;

    while let Some(entry_start) = xml[pos..].find("<entry") {
        let abs_start = pos + entry_start;
        if let Some(entry_end) = xml[abs_start..].find("</entry>") {
            let entry_xml = &xml[abs_start..abs_start + entry_end + 8];

            let title = extract_xml_tag(entry_xml, "title");
            let link = extract_atom_link(entry_xml);
            let summary = extract_xml_tag(entry_xml, "summary")
                .or_else(|| extract_xml_tag(entry_xml, "content"));
            let updated = extract_xml_tag(entry_xml, "updated")
                .or_else(|| extract_xml_tag(entry_xml, "published"));
            let author = extract_xml_tag(entry_xml, "name"); // inside <author><name>

            items.push(RssItem {
                index: items.len(),
                title: strip_cdata(&title.unwrap_or_default()),
                url: link.unwrap_or_default(),
                description: clean_description(&summary.unwrap_or_default()),
                published: updated,
                author,
            });

            pos = abs_start + entry_end + 8;
        } else {
            break;
        }
    }

    Ok(items)
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    if let Some(start) = xml.find(&open) {
        let rest = &xml[start..];
        // Find the end of the opening tag
        if let Some(tag_end) = rest.find('>') {
            let content_start = tag_end + 1;
            let close = format!("</{}>", tag);
            if let Some(close_pos) = rest.find(&close) {
                let content = &rest[content_start..close_pos];
                return Some(content.trim().to_string());
            }
        }
    }
    None
}

fn extract_atom_link(xml: &str) -> Option<String> {
    // Atom links: <link href="..." rel="alternate" />
    let mut pos = 0;
    while let Some(link_start) = xml[pos..].find("<link") {
        let abs = pos + link_start;
        let rest = &xml[abs..];
        if let Some(end) = rest.find('>') {
            let tag = &rest[..end + 1];
            // Prefer rel="alternate" but accept any
            if let Some(href_start) = tag.find("href=\"") {
                let href_rest = &tag[href_start + 6..];
                if let Some(href_end) = href_rest.find('"') {
                    let href = &href_rest[..href_end];
                    if tag.contains("rel=\"alternate\"") || !tag.contains("rel=") {
                        return Some(href.to_string());
                    }
                }
            }
            pos = abs + end + 1;
        } else {
            break;
        }
    }
    // Second pass: accept any link
    let mut pos = 0;
    while let Some(link_start) = xml[pos..].find("<link") {
        let abs = pos + link_start;
        let rest = &xml[abs..];
        if let Some(end) = rest.find('>') {
            let tag = &rest[..end + 1];
            if let Some(href_start) = tag.find("href=\"") {
                let href_rest = &tag[href_start + 6..];
                if let Some(href_end) = href_rest.find('"') {
                    return Some(href_rest[..href_end].to_string());
                }
            }
            pos = abs + end + 1;
        } else {
            break;
        }
    }
    None
}

fn extract_feed_title(xml: &str) -> Option<String> {
    // Get the first <title> that's not inside an <item> or <entry>
    let search_area = if let Some(item_pos) = xml.find("<item").or_else(|| xml.find("<entry")) {
        &xml[..item_pos]
    } else {
        xml
    };
    extract_xml_tag(search_area, "title").map(|t| strip_cdata(&t))
}

fn strip_cdata(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("<![CDATA[") && s.ends_with("]]>") {
        s[9..s.len() - 3].to_string()
    } else {
        strip_html_tags(s)
    }
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' { in_tag = true; }
        else if c == '>' { in_tag = false; }
        else if !in_tag { result.push(c); }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_description(s: &str) -> String {
    let cleaned = strip_cdata(s);
    // Decode HTML entities FIRST, then strip tags
    let decoded = decode_html_entities(&cleaned);
    let stripped = strip_html_tags(&decoded);
    if stripped.len() > 300 {
        format!("{}...", &stripped[..297])
    } else {
        stripped
    }
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
        .replace("&lsquo;", "'")
        .replace("&rsquo;", "'")
        .replace("&ldquo;", "\u{201c}")
        .replace("&rdquo;", "\u{201d}")
}

fn parse_since(since: &str) -> Option<String> {
    // Support "7d", "24h", "2026-03-20"
    if since.ends_with('d') {
        if let Ok(days) = since[..since.len() - 1].parse::<i64>() {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
            return Some(cutoff.to_rfc3339());
        }
    }
    if since.ends_with('h') {
        if let Ok(hours) = since[..since.len() - 1].parse::<i64>() {
            let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
            return Some(cutoff.to_rfc3339());
        }
    }
    // Assume ISO date
    if since.len() >= 10 {
        return Some(format!("{}T00:00:00+00:00", since));
    }
    None
}
