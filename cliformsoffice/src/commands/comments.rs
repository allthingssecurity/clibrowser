use std::path::Path;
use anyhow::Result;
use crate::cli::CommentsArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::CommentsResult;
use crate::output::OutputConfig;

pub fn execute(args: CommentsArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let comments = backend.comments(path)?;

    if out.json {
        let result = CommentsResult {
            count: comments.len(),
            comments,
        };
        out.print_json(&result);
    } else {
        for c in &comments {
            let author = c.author.as_ref().map(|a| format!(" by {}", a)).unwrap_or_default();
            let page = c.page.map(|p| format!(" (page {})", p)).unwrap_or_default();
            out.print_human(&format!("[{}]{}{}: {}", c.index, author, page, c.text));
        }
        out.print_human(&format!("\n{} comments found", comments.len()));
    }
    Ok(0)
}
