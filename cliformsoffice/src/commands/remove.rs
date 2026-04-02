use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use anyhow::Result;
use crate::cli::RemoveArgs;
use crate::error::OfficeError;
use crate::format::{detect_format, FormatKind};
use crate::formats::PageRange;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: RemoveArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    let kind = detect_format(path, format_override)?;
    let output = args.output.as_deref().unwrap_or(&args.file);
    let mut messages = Vec::new();

    // PDF page removal
    if let Some(ref pages) = args.pages {
        if kind != FormatKind::Pdf {
            return Err(OfficeError::NotSupported {
                op: "page removal".into(), format: kind.name().into(),
            }.into());
        }
        let range = PageRange::parse(pages)?;
        let remove_pages = range.to_pages();
        remove_pdf_pages(&args.file, &remove_pages, output)?;
        messages.push(format!("Removed pages {}", pages));
    }

    // Docx comment removal
    if args.comments && kind == FormatKind::Docx {
        let target = if args.pages.is_some() { output } else { output };
        let source = if args.pages.is_some() { output.to_string() } else { args.file.clone() };
        remove_docx_part(&source, target, "word/comments.xml")?;
        messages.push("Removed comments".into());
    }

    // Docx image removal (remove media files)
    if args.images && kind == FormatKind::Docx {
        let source = if messages.is_empty() { args.file.clone() } else { output.to_string() };
        remove_docx_media(&source, output)?;
        messages.push("Removed images".into());
    }

    if messages.is_empty() {
        return Err(OfficeError::Other("specify what to remove: --pages, --comments, --images, or --links".into()).into());
    }

    let result = WriteResult {
        file: output.to_string(),
        format: kind.name().into(),
        message: messages.join("; "),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn remove_pdf_pages(input: &str, pages_to_remove: &[usize], output: &str) -> Result<()> {
    use lopdf::Document;
    let mut doc = Document::load(input)?;
    let all_pages: Vec<u32> = doc.get_pages().keys().cloned().collect();
    // Pages to remove are 1-based
    let remove_set: std::collections::HashSet<u32> = pages_to_remove.iter().map(|&p| p as u32).collect();
    let mut to_delete: Vec<u32> = all_pages.into_iter().filter(|p| remove_set.contains(p)).collect();
    to_delete.sort();
    to_delete.reverse(); // Delete from end to preserve indices
    for page_num in to_delete {
        doc.delete_pages(&[page_num]);
    }
    doc.save(output)?;
    Ok(())
}

fn remove_docx_part(input: &str, output: &str, part_name: &str) -> Result<()> {
    let data = fs::read(input)?;
    let reader = std::io::Cursor::new(&data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = zip::ZipWriter::new(cursor);
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == part_name { continue; } // Skip the part
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(entry.compression());
            writer.start_file(&name, options)?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            writer.write_all(&content)?;
        }
        writer.finish()?;
    }
    fs::write(output, &buf)?;
    Ok(())
}

fn remove_docx_media(input: &str, output: &str) -> Result<()> {
    let data = fs::read(input)?;
    let reader = std::io::Cursor::new(&data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = zip::ZipWriter::new(cursor);
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("word/media/") { continue; }
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(entry.compression());
            writer.start_file(&name, options)?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            writer.write_all(&content)?;
        }
        writer.finish()?;
    }
    fs::write(output, &buf)?;
    Ok(())
}
