use std::path::Path;
use anyhow::Result;
use crate::cli::SearchArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::SearchResult;
use crate::output::OutputConfig;

pub fn execute(args: SearchArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;

    let mut matches = backend.search(path, &args.pattern, args.regex, args.case_sensitive)?;

    if let Some(max) = args.max_results {
        matches.truncate(max);
    }

    if out.json {
        let result = SearchResult {
            count: matches.len(),
            pattern: args.pattern,
            matches,
        };
        out.print_json(&result);
    } else {
        for m in &matches {
            let loc = match (m.page, m.line) {
                (Some(p), Some(l)) => format!("page {}:line {}", p, l),
                (Some(p), None) => format!("page {}", p),
                (None, Some(l)) => format!("line {}", l),
                (None, None) => String::new(),
            };
            let ctx = m.context.as_ref().map(|c| format!(" [{}]", c)).unwrap_or_default();
            out.print_human(&format!("{}{}: {}", loc, ctx, m.text));
        }
        out.print_human(&format!("\n{} matches found", matches.len()));
    }
    Ok(0)
}
