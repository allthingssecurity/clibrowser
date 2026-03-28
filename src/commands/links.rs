use anyhow::Result;
use serde::Serialize;

use crate::cli::LinksArgs;
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct LinksResult {
    count: usize,
    links: Vec<LinkItem>,
}

#[derive(Serialize)]
struct LinkItem {
    index: usize,
    href: String,
    text: String,
}

pub fn execute(args: LinksArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let base_url = if args.absolute {
        session
            .state
            .current_url
            .as_ref()
            .and_then(|u| url::Url::parse(u).ok())
    } else {
        None
    };

    let doc = Document::parse(&html);
    let links = doc.extract_links(&args.selector, base_url.as_ref())?;

    let mut items: Vec<LinkItem> = links
        .into_iter()
        .map(|l| LinkItem {
            index: l.index,
            href: l.href,
            text: l.text,
        })
        .collect();

    // Apply filter
    if let Some(ref filter) = args.filter {
        items.retain(|l| l.href.contains(filter) || l.text.contains(filter));
        // Re-index
        for (i, item) in items.iter_mut().enumerate() {
            item.index = i;
        }
    }

    if out.json {
        out.print_json(&LinksResult {
            count: items.len(),
            links: items,
        });
    } else {
        for item in &items {
            out.print_human(&format!("[{}] {} | {}", item.index, item.href, item.text));
        }
    }

    Ok(0)
}
