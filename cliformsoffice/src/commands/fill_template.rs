use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use anyhow::Result;
use regex::Regex;
use crate::cli::FillTemplateArgs;
use crate::error::OfficeError;
use crate::format::{detect_format, FormatKind};
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: FillTemplateArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    let data: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&args.data)
        .or_else(|_| {
            let content = fs::read_to_string(&args.data)?;
            serde_json::from_str(&content).map_err(anyhow::Error::from)
        })?;

    let kind = detect_format(path, format_override)?;
    let count = match kind {
        FormatKind::Docx => fill_docx(&args.file, &data, &args.output)?,
        FormatKind::Xlsx => fill_xlsx(&args.file, &data, &args.output)?,
        _ => return Err(OfficeError::NotSupported {
            op: "fill-template".into(), format: kind.name().into(),
        }.into()),
    };

    let result = WriteResult {
        file: args.output.clone(), format: kind.name().into(),
        message: format!("{} placeholder(s) filled", count),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn fill_docx(input: &str, data: &serde_json::Map<String, serde_json::Value>, output: &str) -> Result<usize> {
    let file_data = fs::read(input)?;
    let reader = std::io::Cursor::new(&file_data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut buf = Vec::new();
    let mut total = 0usize;
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
                let (replaced, n) = replace_placeholders(&xml, data);
                total += n;
                writer.write_all(replaced.as_bytes())?;
            } else {
                writer.write_all(&content)?;
            }
        }
        writer.finish()?;
    }
    fs::write(output, &buf)?;
    Ok(total)
}

fn fill_xlsx(input: &str, data: &serde_json::Map<String, serde_json::Value>, output: &str) -> Result<usize> {
    let mut book = umya_spreadsheet::reader::xlsx::read(input)
        .map_err(|e| OfficeError::Other(format!("xlsx read error: {}", e)))?;
    let mut total = 0usize;
    for sheet in book.get_sheet_collection_mut().iter_mut() {
        for row in 1..=sheet.get_highest_row() {
            for col in 1..=sheet.get_highest_column() {
                let cell = sheet.get_cell_mut((col, row));
                let val = cell.get_value().to_string();
                if val.contains("{{") {
                    let (replaced, n) = replace_placeholders(&val, data);
                    if n > 0 { total += n; cell.set_value(&replaced); }
                }
            }
        }
    }
    umya_spreadsheet::writer::xlsx::write(&book, output)
        .map_err(|e| OfficeError::Other(format!("xlsx write error: {}", e)))?;
    Ok(total)
}

fn replace_placeholders(text: &str, data: &serde_json::Map<String, serde_json::Value>) -> (String, usize) {
    let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();
    let mut count = 0;
    let result = re.replace_all(text, |caps: &regex::Captures| {
        let key = &caps[1];
        if let Some(val) = data.get(key) {
            count += 1;
            match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }
        } else {
            caps[0].to_string()
        }
    });
    (result.to_string(), count)
}
