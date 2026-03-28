use anyhow::Result;
use serde::Serialize;

use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct MarkdownArgs {
    /// CSS selector to convert (default: article or main or body)
    #[arg(long)]
    pub selector: Option<String>,

    /// Max output length in characters
    #[arg(long)]
    pub max_length: Option<usize>,

    /// Include link URLs inline
    #[arg(long, default_value = "true")]
    pub include_links: bool,
}

#[derive(Serialize)]
struct MarkdownResult {
    url: Option<String>,
    title: Option<String>,
    markdown: String,
    length: usize,
}

pub fn execute(args: MarkdownArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);

    // Get title
    let title = doc
        .select("title")
        .ok()
        .and_then(|els| els.first().map(|e| e.text.clone()));

    // Determine which part of the page to convert
    let selector = args.selector.as_deref().unwrap_or_else(|| {
        // Auto-detect the best content selector
        "article"
    });

    // Try selectors in priority order
    let content_html = if args.selector.is_some() {
        get_inner_html(&doc, selector)
    } else {
        // Auto-detect: article > main > .content > .post-content > body
        get_inner_html(&doc, "article")
            .or_else(|| get_inner_html(&doc, "main"))
            .or_else(|| get_inner_html(&doc, ".content"))
            .or_else(|| get_inner_html(&doc, ".post-content"))
            .or_else(|| get_inner_html(&doc, ".entry-content"))
            .or_else(|| get_inner_html(&doc, "#content"))
            .or_else(|| get_inner_html(&doc, "body"))
    };

    let content = content_html.unwrap_or_else(|| html.clone());
    let mut md = html_to_markdown(&content);

    if let Some(max) = args.max_length {
        if md.len() > max {
            md.truncate(max);
            md.push_str("\n\n... (truncated)");
        }
    }

    if out.json {
        out.print_json(&MarkdownResult {
            url: session.state.current_url.clone(),
            title,
            length: md.len(),
            markdown: md,
        });
    } else {
        println!("{}", md);
    }

    Ok(0)
}

fn get_inner_html(doc: &Document, selector: &str) -> Option<String> {
    doc.select(selector)
        .ok()
        .and_then(|els| els.first().map(|e| e.html.clone()))
}

/// Convert HTML to markdown — handles common elements
fn html_to_markdown(html: &str) -> String {
    let doc = Document::parse(html);
    let mut output = String::new();

    // Simple state machine approach: walk the HTML and convert tags
    convert_node(html, &mut output);

    // Clean up multiple blank lines
    let mut cleaned = String::new();
    let mut prev_blank = false;
    for line in output.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
        prev_blank = is_blank;
    }

    cleaned.trim().to_string()
}

fn convert_node(html: &str, output: &mut String) {
    let mut pos = 0;
    let bytes = html.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            // Find tag end
            let tag_end = find_tag_end(html, pos);
            let tag_content = &html[pos..tag_end];

            if tag_content.starts_with("</") {
                // Closing tag — handle block-level element endings
                let tag_name = extract_tag_name(&tag_content[2..]);
                match tag_name.as_str() {
                    "p" | "div" | "section" | "article" | "main" | "blockquote" => {
                        output.push_str("\n\n");
                    }
                    "li" => output.push('\n'),
                    "tr" => output.push_str(" |\n"),
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        output.push_str("\n\n");
                    }
                    "pre" | "code" => {
                        if tag_name == "pre" {
                            output.push_str("\n```\n\n");
                        }
                    }
                    _ => {}
                }
                pos = tag_end;
            } else if tag_content.starts_with("<!--") {
                // HTML comment — skip
                if let Some(end) = html[pos..].find("-->") {
                    pos = pos + end + 3;
                } else {
                    pos = tag_end;
                }
            } else {
                // Opening tag
                let tag_name = extract_tag_name(&tag_content[1..]);
                match tag_name.as_str() {
                    "h1" => output.push_str("\n# "),
                    "h2" => output.push_str("\n## "),
                    "h3" => output.push_str("\n### "),
                    "h4" => output.push_str("\n#### "),
                    "h5" => output.push_str("\n##### "),
                    "h6" => output.push_str("\n###### "),
                    "p" => output.push_str("\n\n"),
                    "br" | "br/" => output.push('\n'),
                    "hr" | "hr/" => output.push_str("\n---\n"),
                    "strong" | "b" => output.push_str("**"),
                    "em" | "i" => output.push('*'),
                    "code" => output.push('`'),
                    "pre" => output.push_str("\n```\n"),
                    "blockquote" => output.push_str("\n> "),
                    "li" => output.push_str("- "),
                    "ul" | "ol" => output.push('\n'),
                    "a" => {
                        // Extract href and output markdown link start
                        if let Some(href) = extract_attr(tag_content, "href") {
                            output.push('[');
                            // We need to output the link text, then close with ](href)
                            // Find closing </a>
                            if let Some(close) = html[tag_end..].find("</a>") {
                                let link_text = strip_tags(&html[tag_end..tag_end + close]);
                                let link_text = link_text.trim();
                                output.push_str(link_text);
                                output.push_str("](");
                                output.push_str(&href);
                                output.push(')');
                                pos = tag_end + close + 4; // skip past </a>
                                continue;
                            }
                        }
                    }
                    "img" => {
                        if let Some(alt) = extract_attr(tag_content, "alt") {
                            if let Some(src) = extract_attr(tag_content, "src") {
                                output.push_str(&format!("![{}]({})", alt, src));
                            }
                        }
                    }
                    "td" | "th" => output.push_str("| "),
                    "script" | "style" | "noscript" | "nav" | "footer" | "header" => {
                        // Skip these entirely
                        let close_tag = format!("</{}>", tag_name);
                        if let Some(close) = html[tag_end..].find(&close_tag) {
                            pos = tag_end + close + close_tag.len();
                            continue;
                        }
                    }
                    _ => {}
                }

                // Handle self-closing tags
                if tag_content.ends_with("/>") || matches!(tag_name.as_str(), "br" | "hr" | "img" | "input" | "meta" | "link") {
                    // Self-closing, nothing more to do
                }

                pos = tag_end;
            }
        } else {
            // Text content — decode HTML entities and output
            let next_tag = html[pos..].find('<').unwrap_or(html.len() - pos);
            let text = &html[pos..pos + next_tag];
            let decoded = decode_html_entities(text);
            let trimmed = decoded.replace('\n', " ");
            if !trimmed.trim().is_empty() {
                output.push_str(&trimmed);
            }
            pos += next_tag;
        }
    }
}

fn find_tag_end(html: &str, start: usize) -> usize {
    let mut pos = start + 1;
    let mut in_quote = false;
    let mut quote_char = '"';
    while pos < html.len() {
        let c = html.as_bytes()[pos];
        if in_quote {
            if c == quote_char as u8 {
                in_quote = false;
            }
        } else {
            if c == b'"' || c == b'\'' {
                in_quote = true;
                quote_char = c as char;
            } else if c == b'>' {
                return pos + 1;
            }
        }
        pos += 1;
    }
    html.len()
}

fn extract_tag_name(s: &str) -> String {
    s.split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    if let Some(start) = tag.find(&search) {
        let rest = &tag[start + search.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    let search2 = format!("{}='", attr);
    if let Some(start) = tag.find(&search2) {
        let rest = &tag[start + search2.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn strip_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' { in_tag = true; }
        else if c == '>' { in_tag = false; }
        else if !in_tag { result.push(c); }
    }
    result
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
