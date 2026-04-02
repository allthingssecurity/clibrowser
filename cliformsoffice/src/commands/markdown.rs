use std::path::Path;
use anyhow::Result;
use crate::cli::MarkdownArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::{get_backend, PageRange};
use crate::models::MarkdownResult;
use crate::output::OutputConfig;

pub fn execute(args: MarkdownArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;

    let page_range = args.pages.as_ref().map(|p| PageRange::parse(p)).transpose()?;
    let mut md = backend.markdown(path, page_range.as_ref())?;

    if let Some(max) = args.max_length {
        if md.len() > max {
            md.truncate(max);
            md.push_str("\n\n...(truncated)");
        }
    }

    if out.json {
        let result = MarkdownResult {
            length: md.len(),
            markdown: md,
        };
        out.print_json(&result);
    } else {
        out.print_human(&md);
    }
    Ok(0)
}
