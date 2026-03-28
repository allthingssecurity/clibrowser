use anyhow::Result;
use serde::Serialize;

use crate::cli::GetArgs;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct SitemapArgs {
    /// Site URL or direct sitemap.xml URL
    pub url: String,

    /// Max URLs to return
    #[arg(long, short = 'n', default_value = "50")]
    pub max_urls: usize,

    /// Filter URLs by substring
    #[arg(long)]
    pub filter: Option<String>,

    /// Follow sitemap index files (sitemapindex)
    #[arg(long)]
    pub recursive: bool,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(Serialize)]
struct SitemapResult {
    source: String,
    count: usize,
    urls: Vec<SitemapUrl>,
}

#[derive(Serialize, Clone)]
struct SitemapUrl {
    index: usize,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lastmod: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changefreq: Option<String>,
}

pub async fn execute(mut args: SitemapArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;

    // If URL doesn't end with sitemap.xml, try to find it
    let sitemap_url = if args.url.ends_with("sitemap.xml") || args.url.contains("sitemap") {
        args.url.clone()
    } else {
        // Try common sitemap locations
        let base = args.url.trim_end_matches('/');
        format!("{}/sitemap.xml", base)
    };

    let mut all_urls: Vec<SitemapUrl> = Vec::new();
    let mut sitemaps_to_fetch = vec![sitemap_url.clone()];

    while let Some(url) = sitemaps_to_fetch.pop() {
        let get_args = GetArgs {
            url: url.clone(),
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

        match crate::http::fetch(&get_args, session).await {
            Ok(response) => {
                if response.body.contains("<sitemapindex") {
                    // This is a sitemap index — extract child sitemaps
                    let child_sitemaps = extract_sitemap_index(&response.body);
                    if args.recursive {
                        if !out.quiet && !out.json {
                            out.print_human(&format!("Found sitemap index with {} child sitemaps", child_sitemaps.len()));
                        }
                        sitemaps_to_fetch.extend(child_sitemaps);
                    } else {
                        // Just list the child sitemaps as URLs
                        for (i, sm) in child_sitemaps.into_iter().enumerate() {
                            all_urls.push(SitemapUrl {
                                index: i,
                                url: sm,
                                lastmod: None,
                                priority: None,
                                changefreq: None,
                            });
                        }
                    }
                } else {
                    // Regular sitemap — extract URLs
                    let urls = parse_sitemap_urls(&response.body);
                    all_urls.extend(urls);
                }
            }
            Err(e) => {
                if !out.quiet {
                    eprintln!("Warning: could not fetch {}: {}", url, e);
                }
                // Try robots.txt as fallback
                if url == sitemap_url {
                    let base = args.url.trim_end_matches('/');
                    let robots_url = format!("{}/robots.txt", base);
                    if let Ok(found) = find_sitemap_in_robots(&robots_url, session, args.stealth).await {
                        sitemaps_to_fetch.extend(found);
                    }
                }
            }
        }

        if all_urls.len() >= args.max_urls {
            break;
        }
    }

    // Apply filter
    if let Some(ref filter) = args.filter {
        let f = filter.to_lowercase();
        all_urls.retain(|u| u.url.to_lowercase().contains(&f));
    }

    // Truncate and re-index
    all_urls.truncate(args.max_urls);
    for (i, url) in all_urls.iter_mut().enumerate() {
        url.index = i;
    }

    if out.json {
        out.print_json(&SitemapResult {
            source: sitemap_url,
            count: all_urls.len(),
            urls: all_urls,
        });
    } else {
        out.print_human(&format!("Sitemap: {} ({} URLs)\n", sitemap_url, all_urls.len()));
        for url in &all_urls {
            let meta = [
                url.lastmod.as_deref(),
                url.priority.as_deref(),
                url.changefreq.as_deref(),
            ]
            .iter()
            .filter_map(|v| *v)
            .collect::<Vec<_>>()
            .join(", ");

            if meta.is_empty() {
                out.print_human(&format!("[{}] {}", url.index, url.url));
            } else {
                out.print_human(&format!("[{}] {} ({})", url.index, url.url, meta));
            }
        }
    }

    Ok(0)
}

fn parse_sitemap_urls(xml: &str) -> Vec<SitemapUrl> {
    let mut urls = Vec::new();
    let mut pos = 0;

    while let Some(start) = xml[pos..].find("<url>").or_else(|| xml[pos..].find("<url ")) {
        let abs_start = pos + start;
        if let Some(end) = xml[abs_start..].find("</url>") {
            let url_xml = &xml[abs_start..abs_start + end + 6];

            let loc = extract_tag(url_xml, "loc");
            let lastmod = extract_tag(url_xml, "lastmod");
            let priority = extract_tag(url_xml, "priority");
            let changefreq = extract_tag(url_xml, "changefreq");

            if let Some(loc) = loc {
                urls.push(SitemapUrl {
                    index: urls.len(),
                    url: loc.trim().to_string(),
                    lastmod,
                    priority,
                    changefreq,
                });
            }

            pos = abs_start + end + 6;
        } else {
            break;
        }
    }

    urls
}

fn extract_sitemap_index(xml: &str) -> Vec<String> {
    let mut sitemaps = Vec::new();
    let mut pos = 0;

    while let Some(start) = xml[pos..].find("<sitemap>").or_else(|| xml[pos..].find("<sitemap ")) {
        let abs_start = pos + start;
        if let Some(end) = xml[abs_start..].find("</sitemap>") {
            let sm_xml = &xml[abs_start..abs_start + end + 10];
            if let Some(loc) = extract_tag(sm_xml, "loc") {
                sitemaps.push(loc.trim().to_string());
            }
            pos = abs_start + end + 10;
        } else {
            break;
        }
    }

    sitemaps
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let content_start = start + open.len();
        if let Some(end) = xml[content_start..].find(&close) {
            return Some(xml[content_start..content_start + end].to_string());
        }
    }
    None
}

async fn find_sitemap_in_robots(robots_url: &str, session: &mut Session, stealth: bool) -> Result<Vec<String>> {
    let get_args = GetArgs {
        url: robots_url.to_string(),
        method: "GET".to_string(),
        headers: vec![],
        data: None,
        data_json: None,
        no_follow: false,
        max_redirects: 5,
        timeout: 10,
        user_agent: None,
        stealth,
    };

    let response = crate::http::fetch(&get_args, session).await?;
    let sitemaps: Vec<String> = response
        .body
        .lines()
        .filter(|line| line.to_lowercase().starts_with("sitemap:"))
        .filter_map(|line| line.split_once(':').map(|(_, url)| url.trim().to_string()))
        .collect();

    Ok(sitemaps)
}
