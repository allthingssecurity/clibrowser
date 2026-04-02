use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use anyhow::Result;
use crate::cli::AddSectionArgs;
use crate::error::OfficeError;
use crate::format::{detect_format, FormatKind};
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: AddSectionArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    let kind = detect_format(path, format_override)?;
    if kind != FormatKind::Docx {
        return Err(OfficeError::NotSupported {
            op: "add-section".into(), format: kind.name().into(),
        }.into());
    }

    let md_content = if let Some(ref content) = args.content {
        content.clone()
    } else if let Some(ref from) = args.from {
        fs::read_to_string(from)?
    } else {
        return Err(OfficeError::Other("provide content or --from file".into()).into());
    };

    // Convert markdown to XML paragraphs
    let xml_paragraphs = md_to_docx_xml(&md_content);

    // Read the docx ZIP, inject paragraphs into document.xml
    let data = fs::read(&args.file)?;
    let reader = std::io::Cursor::new(&data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = zip::ZipWriter::new(cursor);
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(entry.compression());
            writer.start_file(&name, options)?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            if name == "word/document.xml" {
                let xml = String::from_utf8_lossy(&content).to_string();
                // Insert before closing </w:body>
                let new_xml = xml.replace("</w:body>", &format!("{}</w:body>", xml_paragraphs));
                writer.write_all(new_xml.as_bytes())?;
            } else {
                writer.write_all(&content)?;
            }
        }
        writer.finish()?;
    }
    fs::write(&args.file, &buf)?;

    let result = WriteResult {
        file: args.file, format: "docx".into(),
        message: "Section added".into(),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn md_to_docx_xml(md: &str) -> String {
    use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};
    let parser = Parser::new(md);
    let mut xml = String::new();
    let mut in_para = false;
    let mut heading_level: Option<usize> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1, HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3, _ => 4,
                });
                xml.push_str("<w:p><w:pPr><w:pStyle w:val=\"Heading");
                xml.push_str(&heading_level.unwrap().to_string());
                xml.push_str("\"/></w:pPr>");
            }
            Event::End(TagEnd::Heading(_)) => {
                xml.push_str("</w:p>");
                heading_level = None;
            }
            Event::Start(Tag::Paragraph) => {
                in_para = true;
                xml.push_str("<w:p>");
            }
            Event::End(TagEnd::Paragraph) => {
                xml.push_str("</w:p>");
                in_para = false;
            }
            Event::Text(text) => {
                if !in_para && heading_level.is_none() {
                    xml.push_str("<w:p>");
                }
                xml.push_str("<w:r><w:t xml:space=\"preserve\">");
                xml.push_str(&escape_xml(&text));
                xml.push_str("</w:t></w:r>");
                if !in_para && heading_level.is_none() {
                    xml.push_str("</w:p>");
                }
            }
            _ => {}
        }
    }
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
