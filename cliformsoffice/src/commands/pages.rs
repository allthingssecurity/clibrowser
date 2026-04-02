use std::path::Path;
use anyhow::Result;
use crate::cli::PagesArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::PagesResult;
use crate::output::OutputConfig;

pub fn execute(args: PagesArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let pages = backend.pages(path)?;

    if out.json {
        let result = PagesResult {
            count: pages.len(),
            pages,
        };
        out.print_json(&result);
    } else {
        for page in &pages {
            let extra = page.word_count
                .map(|w| format!(" ({} words)", w))
                .unwrap_or_default();
            out.print_human(&format!("[{}] {}{}", page.index, page.name, extra));
        }
    }
    Ok(0)
}
