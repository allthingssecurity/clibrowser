use std::path::Path;
use anyhow::Result;
use regex::Regex;
use crate::cli::RedactArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::{RedactMatch, RedactResult};
use crate::output::OutputConfig;

pub fn execute(args: RedactArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let text = backend.text(path, None)?;

    let mut patterns: Vec<(&str, Regex)> = Vec::new();
    if args.email {
        patterns.push(("email", Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}")?));
    }
    if args.phone {
        patterns.push(("phone", Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b")?));
    }
    if args.ssn {
        patterns.push(("ssn", Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")?));
    }
    if args.credit_card {
        patterns.push(("credit_card", Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")?));
    }
    for p in &args.patterns {
        patterns.push(("custom", Regex::new(p)?));
    }

    if patterns.is_empty() {
        // Default: all built-in
        patterns.push(("email", Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}")?));
        patterns.push(("phone", Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b")?));
        patterns.push(("ssn", Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")?));
        patterns.push(("credit_card", Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")?));
    }

    let mut matches = Vec::new();
    for (ptype, re) in &patterns {
        for m in re.find_iter(&text) {
            // Find approximate line number
            let prefix = &text[..m.start()];
            let line = prefix.matches('\n').count() + 1;
            matches.push(RedactMatch {
                text: m.as_str().to_string(),
                pattern_type: ptype.to_string(),
                location: Some(format!("line {}", line)),
            });
        }
    }

    let redacted = if !args.dry_run && !matches.is_empty() {
        // Perform actual replacement using the replace command logic
        let output_path = args.output.as_deref().unwrap_or(&args.file);
        for (_, re) in &patterns {
            let find = re.as_str();
            // Use replace command internally
            crate::commands::replace::execute(
                crate::cli::ReplaceArgs {
                    file: args.file.clone(),
                    find: find.to_string(),
                    replace_with: "[REDACTED]".to_string(),
                    regex: true,
                    all: true,
                    output: Some(output_path.to_string()),
                },
                &OutputConfig { json: false, quiet: true },
                format_override,
            ).ok();
        }
        true
    } else {
        false
    };

    let result = RedactResult {
        count: matches.len(),
        matches,
        redacted,
    };

    if out.json {
        out.print_json(&result);
    } else {
        for m in &result.matches {
            let loc = m.location.as_deref().unwrap_or("?");
            out.print_human(&format!("[{}] {} at {}", m.pattern_type, m.text, loc));
        }
        out.print_human(&format!("\n{} PII match(es) found{}", result.count,
            if result.redacted { ", redacted" } else { "" }));
    }
    Ok(0)
}
