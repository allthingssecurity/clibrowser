use anyhow::Result;
use serde::Serialize;

use crate::cli::TextArgs;
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct TextResult {
    text: String,
    length: usize,
    truncated: bool,
}

pub fn execute(args: TextArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);
    let mut text = doc.extract_text(&args.selector)?;

    if args.strip {
        // Collapse multiple whitespace into single spaces, trim lines
        text = text
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut truncated = false;
    if let Some(max) = args.max_length {
        if text.len() > max {
            text.truncate(max);
            truncated = true;
        }
    }

    if out.json {
        out.print_json(&TextResult {
            length: text.len(),
            text,
            truncated,
        });
    } else {
        out.print_human(&text);
        if truncated {
            out.print_human("... (truncated)");
        }
    }

    Ok(0)
}
