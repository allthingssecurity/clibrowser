use anyhow::Result;
use serde::Serialize;

use crate::cli::SelectArgs;
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct SelectResult {
    count: usize,
    results: Vec<SelectItem>,
}

#[derive(Serialize)]
struct SelectItem {
    index: usize,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attr_value: Option<String>,
    attributes: std::collections::HashMap<String, String>,
}

pub fn execute(args: SelectArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);
    let elements = doc.select(&args.selector)?;

    let mut items: Vec<SelectItem> = elements
        .iter()
        .map(|el| {
            let attr_value = args.attr.as_ref().and_then(|a| el.attributes.get(a).cloned());
            SelectItem {
                index: el.index,
                text: el.text.clone(),
                html: if args.html { Some(el.html.clone()) } else { None },
                attr_value,
                attributes: el.attributes.clone(),
            }
        })
        .collect();

    // Apply index filter
    if let Some(idx) = args.index {
        if idx >= items.len() {
            return Err(BrowserError::IndexOutOfRange {
                index: idx,
                count: items.len(),
            }
            .into());
        }
        items = vec![items.remove(idx)];
    }

    // Apply first filter
    if args.first {
        items.truncate(1);
    }

    // Apply limit
    if let Some(limit) = args.limit {
        items.truncate(limit);
    }

    if out.json {
        out.print_json(&SelectResult {
            count: items.len(),
            results: items,
        });
    } else {
        for item in &items {
            let display = if let Some(ref av) = item.attr_value {
                av.clone()
            } else if args.html {
                item.html.as_deref().unwrap_or("").to_string()
            } else {
                item.text.clone()
            };
            out.print_human(&format!("[{}] {}", item.index, display));
        }
    }

    Ok(0)
}
