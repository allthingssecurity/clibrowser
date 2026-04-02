use std::path::Path;
use anyhow::Result;
use crate::cli::StylesArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::StylesResult;
use crate::output::OutputConfig;

pub fn execute(args: StylesArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let mut styles = backend.styles(path)?;

    if let Some(ref filter_type) = args.style_type {
        styles.retain(|s| s.style_type == *filter_type);
    }

    if out.json {
        let result = StylesResult {
            count: styles.len(),
            styles,
        };
        out.print_json(&result);
    } else {
        for s in &styles {
            out.print_human(&format!("{} ({})", s.name, s.style_type));
        }
        out.print_human(&format!("\n{} styles found", styles.len()));
    }
    Ok(0)
}
