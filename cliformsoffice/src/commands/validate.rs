use std::path::Path;
use anyhow::Result;
use regex::Regex;
use crate::cli::ValidateArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::{ValidationIssue, ValidateResult};
use crate::output::OutputConfig;

pub fn execute(args: ValidateArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file).into()); }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let mut issues = Vec::new();

    // Check required fields in text/headings
    if let Some(ref fields) = args.required_fields {
        let text = backend.text(path, None).unwrap_or_default().to_lowercase();
        let toc = backend.toc(path).unwrap_or_default();
        let headings: Vec<String> = toc.iter().map(|e| e.text.to_lowercase()).collect();

        for field in fields.split(',') {
            let f = field.trim().to_lowercase();
            if !text.contains(&f) && !headings.iter().any(|h| h.contains(&f)) {
                issues.push(ValidationIssue {
                    rule: "required_field".into(),
                    message: format!("Required field '{}' not found", field.trim()),
                    severity: "error".into(),
                });
            }
        }
    }

    // Check page count
    if let Some(max) = args.max_pages {
        let info = backend.info(path)?;
        let page_count = info.pages.or(info.slides).unwrap_or(0);
        if page_count > max {
            issues.push(ValidationIssue {
                rule: "max_pages".into(),
                message: format!("Document has {} pages, max allowed is {}", page_count, max),
                severity: "error".into(),
            });
        }
    }

    // Check empty cells in tables
    if args.no_empty_cells {
        let tables = backend.tables(path, None).unwrap_or_default();
        for (ti, table) in tables.iter().enumerate() {
            for (ri, row) in table.data.iter().enumerate() {
                for (ci, cell) in row.iter().enumerate() {
                    if cell.trim().is_empty() {
                        issues.push(ValidationIssue {
                            rule: "no_empty_cells".into(),
                            message: format!("Empty cell at table {}, row {}, col {}", ti, ri, ci),
                            severity: "warning".into(),
                        });
                    }
                }
            }
        }
    }

    // Check links are valid URLs
    if args.check_links {
        let url_re = Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
        let links = backend.links(path).unwrap_or_default();
        for link in &links {
            if !url_re.is_match(&link.url) {
                issues.push(ValidationIssue {
                    rule: "valid_links".into(),
                    message: format!("Invalid URL: {}", link.url),
                    severity: "warning".into(),
                });
            }
        }
    }

    let valid = !issues.iter().any(|i| i.severity == "error");
    let result = ValidateResult { valid, issues };

    if out.json {
        out.print_json(&result);
    } else {
        for i in &result.issues {
            out.print_human(&format!("[{}] {}: {}", i.severity.to_uppercase(), i.rule, i.message));
        }
        out.print_human(&format!("\nValidation: {}", if result.valid { "PASS" } else { "FAIL" }));
    }
    Ok(0)
}
