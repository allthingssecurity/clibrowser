use std::path::Path;
use std::io::Read;
use anyhow::Result;
use crate::cli::HeadersFootersArgs;
use crate::error::OfficeError;
use crate::format::{detect_format, FormatKind};
use crate::output::OutputConfig;

pub fn execute(args: HeadersFootersArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    // Writing headers/footers is complex — only support reading
    if args.set_header.is_some() || args.set_footer.is_some() {
        return Err(OfficeError::NotSupported {
            op: "set headers/footers".into(),
            format: "docx".into(),
        }.into());
    }

    let kind = detect_format(path, format_override)?;
    if kind != FormatKind::Docx {
        return Err(OfficeError::NotSupported {
            op: "headers-footers".into(),
            format: kind.name().into(),
        }.into());
    }

    let data = std::fs::read(&args.file)?;
    let reader = std::io::Cursor::new(&data);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut header_text = String::new();
    let mut footer_text = String::new();

    // Try common header/footer file names
    for name in &["word/header1.xml", "word/header2.xml", "word/header3.xml"] {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            let text = extract_text_from_xml(&content);
            if !text.is_empty() {
                if !header_text.is_empty() { header_text.push('\n'); }
                header_text.push_str(&text);
            }
        }
    }

    for name in &["word/footer1.xml", "word/footer2.xml", "word/footer3.xml"] {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            let text = extract_text_from_xml(&content);
            if !text.is_empty() {
                if !footer_text.is_empty() { footer_text.push('\n'); }
                footer_text.push_str(&text);
            }
        }
    }

    if out.json {
        let result = serde_json::json!({
            "header": if header_text.is_empty() { None } else { Some(&header_text) },
            "footer": if footer_text.is_empty() { None } else { Some(&footer_text) },
        });
        out.print_json(&result);
    } else {
        if !header_text.is_empty() { out.print_human(&format!("Header: {}", header_text)); }
        else { out.print_human("Header: (none)"); }
        if !footer_text.is_empty() { out.print_human(&format!("Footer: {}", footer_text)); }
        else { out.print_human("Footer: (none)"); }
    }
    Ok(0)
}

fn extract_text_from_xml(xml: &str) -> String {
    // Simple extraction: get text between <w:t> tags
    let mut result = String::new();
    let re = regex::Regex::new(r"<w:t[^>]*>([^<]*)</w:t>").unwrap();
    for cap in re.captures_iter(xml) {
        result.push_str(&cap[1]);
    }
    result
}
