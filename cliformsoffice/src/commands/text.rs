use std::path::Path;
use anyhow::Result;
use crate::cli::TextArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::{get_backend, PageRange};
use crate::models::TextResult;
use crate::output::OutputConfig;

pub fn execute(args: TextArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;

    let page_range = args.pages.as_ref().map(|p| PageRange::parse(p)).transpose()?;
    let mut text = backend.text(path, page_range.as_ref())?;

    if args.strip {
        text = text.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }

    if let Some(max) = args.max_length {
        if text.len() > max {
            text.truncate(max);
            text.push_str("...");
        }
    }

    if out.json {
        let result = TextResult {
            length: text.len(),
            text,
        };
        out.print_json(&result);
    } else {
        out.print_human(&text);
    }
    Ok(0)
}
