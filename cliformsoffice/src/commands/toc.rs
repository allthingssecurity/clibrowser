use std::path::Path;
use anyhow::Result;
use crate::cli::TocArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::TocResult;
use crate::output::OutputConfig;

pub fn execute(args: TocArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let mut entries = backend.toc(path)?;

    if let Some(max_depth) = args.depth {
        entries.retain(|e| e.level <= max_depth);
    }

    if out.json {
        let result = TocResult {
            count: entries.len(),
            entries,
        };
        out.print_json(&result);
    } else {
        for e in &entries {
            let indent = "  ".repeat(e.level.saturating_sub(1));
            let page = e.page.map(|p| format!(" (p.{})", p)).unwrap_or_default();
            out.print_human(&format!("{}{}{}", indent, e.text, page));
        }
    }
    Ok(0)
}
