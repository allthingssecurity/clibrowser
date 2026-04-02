use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use anyhow::Result;
use regex::Regex;
use crate::cli::ReplaceArgs;
use crate::error::OfficeError;
use crate::format::{detect_format, FormatKind};
use crate::models::ReplaceResult;
use crate::output::OutputConfig;

pub fn execute(args: ReplaceArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    let kind = detect_format(path, format_override)?;
    let output_path = args.output.as_deref().unwrap_or(&args.file);
    let count;

    match kind {
        FormatKind::Docx => {
            count = replace_in_docx(&args.file, &args.find, &args.replace_with, args.regex, args.all, output_path)?;
        }
        FormatKind::Xlsx => {
            count = replace_in_xlsx(&args.file, &args.find, &args.replace_with, args.regex, args.all, output_path)?;
        }
        _ => {
            return Err(OfficeError::NotSupported {
                op: "replace".into(), format: kind.name().into(),
            }.into());
        }
    }

    let result = ReplaceResult {
        file: output_path.to_string(),
        replacements: count,
        message: format!("{} replacement(s) made", count),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn replace_in_docx(input: &str, find: &str, replace_with: &str, use_regex: bool, all: bool, output: &str) -> Result<usize> {
    let data = fs::read(input)?;
    let reader = std::io::Cursor::new(&data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = zip::ZipWriter::new(cursor);
        let mut count = 0usize;
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
                let (replaced, n) = do_replace(&xml, find, replace_with, use_regex, all);
                count += n;
                writer.write_all(replaced.as_bytes())?;
            } else {
                writer.write_all(&content)?;
            }
        }
        writer.finish()?;
        fs::write(output, &buf)?;
        return Ok(count);
    }
}

fn replace_in_xlsx(input: &str, find: &str, replace_with: &str, use_regex: bool, all: bool, output: &str) -> Result<usize> {
    let mut book = umya_spreadsheet::reader::xlsx::read(input)
        .map_err(|e| OfficeError::Other(format!("xlsx read error: {}", e)))?;
    let mut count = 0usize;
    for sheet in book.get_sheet_collection_mut().iter_mut() {
        for row in 1..=sheet.get_highest_row() {
            for col in 1..=sheet.get_highest_column() {
                let cell = sheet.get_cell_mut((col, row));
                let val = cell.get_value().to_string();
                let (replaced, n) = do_replace(&val, find, replace_with, use_regex, all);
                if n > 0 {
                    count += n;
                    cell.set_value(&replaced);
                }
            }
        }
    }
    umya_spreadsheet::writer::xlsx::write(&book, output)
        .map_err(|e| OfficeError::Other(format!("xlsx write error: {}", e)))?;
    Ok(count)
}

fn do_replace(text: &str, find: &str, replace_with: &str, use_regex: bool, all: bool) -> (String, usize) {
    if use_regex {
        if let Ok(re) = Regex::new(find) {
            let mut count = 0;
            let result = if all {
                let r = re.replace_all(text, replace_with);
                count = re.find_iter(text).count();
                r.to_string()
            } else {
                if re.is_match(text) { count = 1; }
                re.replace(text, replace_with).to_string()
            };
            (result, count)
        } else {
            (text.to_string(), 0)
        }
    } else {
        let count = if all { text.matches(find).count() } else if text.contains(find) { 1 } else { 0 };
        let result = if all {
            text.replace(find, replace_with)
        } else {
            text.replacen(find, replace_with, 1)
        };
        (result, count)
    }
}
