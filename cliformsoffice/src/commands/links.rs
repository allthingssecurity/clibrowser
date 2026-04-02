use std::path::Path;
use anyhow::Result;
use crate::cli::LinksArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::LinksResult;
use crate::output::OutputConfig;

pub fn execute(args: LinksArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let links = backend.links(path)?;

    if out.json {
        let result = LinksResult {
            count: links.len(),
            links,
        };
        out.print_json(&result);
    } else {
        for l in &links {
            let text = l.text.as_ref().map(|t| format!(" \"{}\"", t)).unwrap_or_default();
            let page = l.page.map(|p| format!(" (page {})", p)).unwrap_or_default();
            out.print_human(&format!("[{}] {}{}{}", l.index, l.url, text, page));
        }
        out.print_human(&format!("\n{} links found", links.len()));
    }
    Ok(0)
}
