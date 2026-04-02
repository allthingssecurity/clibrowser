use std::path::Path;
use anyhow::Result;
use crate::cli::SummaryArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::SummaryResult;
use crate::output::OutputConfig;

pub fn execute(args: SummaryArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;

    let info = backend.info(path)?;
    let toc_entries = backend.toc(path).unwrap_or_default();
    let text = backend.text(path, None).unwrap_or_default();
    let tables = backend.tables(path, None).unwrap_or_default();
    let images = backend.images(path, None).unwrap_or_default();
    let links = backend.links(path).unwrap_or_default();
    let comments = backend.comments(path).unwrap_or_default();

    let outline: Vec<String> = toc_entries.iter().map(|e| {
        format!("{}{}", "  ".repeat(e.level.saturating_sub(1)), e.text)
    }).collect();

    // Build section previews from text
    let preview_len = args.max_preview;
    let sections: Vec<String> = if text.len() <= preview_len {
        vec![text.clone()]
    } else {
        text.split("\n\n")
            .take(5)
            .map(|s| {
                let trimmed = s.trim();
                if trimmed.len() > preview_len {
                    format!("{}...", &trimmed[..preview_len])
                } else {
                    trimmed.to_string()
                }
            })
            .filter(|s| !s.is_empty())
            .collect()
    };

    let word_count = info.word_count.unwrap_or_else(|| text.split_whitespace().count());

    let result = SummaryResult {
        title: info.title,
        outline,
        section_previews: sections,
        table_count: tables.len(),
        image_count: images.len(),
        link_count: links.len(),
        comment_count: comments.len(),
        word_count,
    };

    if out.json {
        out.print_json(&result);
    } else {
        if let Some(ref t) = result.title { out.print_human(&format!("Title: {}", t)); }
        if !result.outline.is_empty() {
            out.print_human("\nOutline:");
            for e in &result.outline { out.print_human(&format!("  {}", e)); }
        }
        out.print_human(&format!("\nWords: {}  Tables: {}  Images: {}  Links: {}  Comments: {}",
            result.word_count, result.table_count, result.image_count,
            result.link_count, result.comment_count));
        if !result.section_previews.is_empty() {
            out.print_human("\nPreview:");
            for s in &result.section_previews { out.print_human(s); }
        }
    }
    Ok(0)
}
