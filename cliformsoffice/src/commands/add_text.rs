use std::path::Path;
use anyhow::Result;
use crate::cli::AddTextArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: AddTextArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file.clone()).into());
    }

    let kind = detect_format(path, format_override)?;

    match kind {
        crate::format::FormatKind::Docx => add_text_docx(path, &args, out),
        crate::format::FormatKind::Xlsx => add_text_xlsx(path, &args, out),
        _ => Err(OfficeError::NotSupported {
            op: "add-text".into(),
            format: kind.name().into(),
        }.into()),
    }
}

fn add_text_docx(path: &Path, args: &AddTextArgs, out: &OutputConfig) -> Result<i32> {
    // Read existing, modify, and write back using docx-rs
    let data = std::fs::read(path)?;
    let mut doc = docx_rs::read_docx(&data).map_err(|e| {
        OfficeError::FormatError {
            file: path.display().to_string(),
            detail: format!("failed to parse DOCX: {}", e),
        }
    })?;

    let mut run = docx_rs::Run::new().add_text(&args.content);
    if args.bold {
        run = run.bold();
    }
    if args.italic {
        run = run.italic();
    }

    let para = docx_rs::Paragraph::new().add_run(run);
    doc = doc.add_paragraph(para);

    let file = std::fs::File::create(path)?;
    doc.build().pack(file)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "docx".into(),
        message: "Text added to document".into(),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn add_text_xlsx(path: &Path, args: &AddTextArgs, out: &OutputConfig) -> Result<i32> {
    let mut workbook = umya_spreadsheet::reader::xlsx::read(path).map_err(|e| {
        OfficeError::FormatError {
            file: path.display().to_string(),
            detail: format!("failed to open XLSX: {}", e),
        }
    })?;

    // Find the target sheet (default first)
    let sheet = workbook.get_sheet_mut(&0usize).ok_or_else(|| {
        OfficeError::Other("failed to get sheet".into())
    })?;

    // Find first empty row in column A
    let mut row = 1u32;
    loop {
        let cell = sheet.get_cell((1u32, row));
        if cell.is_none() || cell.unwrap().get_value().is_empty() {
            break;
        }
        row += 1;
        if row > 1_000_000 {
            break;
        }
    }

    sheet.get_cell_mut((1u32, row)).set_value(&args.content);

    umya_spreadsheet::writer::xlsx::write(&workbook, path).map_err(|e| {
        OfficeError::Io(format!("failed to save: {}", e))
    })?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "xlsx".into(),
        message: format!("Text added at row {}", row),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
