use std::path::Path;
use anyhow::Result;
use crate::cli::InfoArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::output::OutputConfig;

pub fn execute(args: InfoArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let info = backend.info(path)?;

    if out.json {
        out.print_json(&info);
    } else {
        out.print_human(&format!("File:       {}", info.file));
        out.print_human(&format!("Format:     {}", info.format));
        out.print_human(&format!("Size:       {} bytes", info.file_size));
        if let Some(p) = info.pages { out.print_human(&format!("Pages:      {}", p)); }
        if let Some(s) = info.slides { out.print_human(&format!("Slides:     {}", s)); }
        if let Some(ref sheets) = info.sheets { out.print_human(&format!("Sheets:     {} ({})", sheets.len(), sheets.join(", "))); }
        if let Some(w) = info.word_count { out.print_human(&format!("Words:      {}", w)); }
        if let Some(c) = info.char_count { out.print_human(&format!("Characters: {}", c)); }
        if let Some(ref t) = info.title { out.print_human(&format!("Title:      {}", t)); }
        if let Some(ref a) = info.author { out.print_human(&format!("Author:     {}", a)); }
        if let Some(ref s) = info.subject { out.print_human(&format!("Subject:    {}", s)); }
        if let Some(ref c) = info.created { out.print_human(&format!("Created:    {}", c)); }
        if let Some(ref m) = info.modified { out.print_human(&format!("Modified:   {}", m)); }
    }
    Ok(0)
}
