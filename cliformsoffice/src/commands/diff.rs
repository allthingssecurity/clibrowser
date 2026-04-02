use std::path::Path;
use anyhow::Result;
use similar::{ChangeTag, TextDiff};
use crate::cli::DiffArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::{DiffResult, DiffLine};
use crate::output::OutputConfig;

pub fn execute(args: DiffArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path1 = Path::new(&args.file1);
    let path2 = Path::new(&args.file2);
    if !path1.exists() { return Err(OfficeError::FileNotFound(args.file1).into()); }
    if !path2.exists() { return Err(OfficeError::FileNotFound(args.file2).into()); }

    let kind1 = detect_format(path1, format_override)?;
    let kind2 = detect_format(path2, format_override)?;
    let b1 = get_backend(kind1)?;
    let b2 = get_backend(kind2)?;

    let mut text1 = b1.text(path1, None)?;
    let mut text2 = b2.text(path2, None)?;

    if args.ignore_whitespace {
        text1 = normalize_ws(&text1);
        text2 = normalize_ws(&text2);
    }

    let diff = TextDiff::from_lines(&text1, &text2);
    let (mut added, mut removed): (usize, usize) = (0, 0);
    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Insert => { added += 1; "+" }
            ChangeTag::Delete => { removed += 1; "-" }
            ChangeTag::Equal => continue,
        };
        lines.push(DiffLine {
            tag: tag.to_string(),
            text: change.value().trim_end_matches('\n').to_string(),
        });
    }

    // Approximate "changed" as min of added/removed pairs
    let changed = added.min(removed);
    let net_added = added.saturating_sub(changed);
    let net_removed = removed.saturating_sub(changed);

    let summary = format!("{} added, {} removed, {} changed lines", net_added, net_removed, changed);

    let result = DiffResult { added, removed, changed, summary: summary.clone(), lines };

    if out.json {
        out.print_json(&result);
    } else {
        for l in &result.lines {
            out.print_human(&format!("{} {}", l.tag, l.text));
        }
        out.print_human(&format!("\n{}", summary));
    }
    Ok(0)
}

fn normalize_ws(s: &str) -> String {
    s.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>().join("\n")
}
