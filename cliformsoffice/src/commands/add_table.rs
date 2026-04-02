use std::path::Path;
use anyhow::Result;
use crate::cli::AddTableArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: AddTableArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file.clone()).into());
    }

    let source_path = Path::new(&args.from);
    if !source_path.exists() {
        return Err(OfficeError::FileNotFound(args.from.clone()).into());
    }

    let kind = detect_format(path, format_override)?;

    // Read table data from CSV/JSON source
    let rows = read_table_data(source_path, args.headers)?;

    match kind {
        crate::format::FormatKind::Docx => add_table_docx(path, &rows, out),
        crate::format::FormatKind::Xlsx => add_table_xlsx(path, &rows, out),
        _ => Err(OfficeError::NotSupported {
            op: "add-table".into(),
            format: kind.name().into(),
        }.into()),
    }
}

fn read_table_data(source: &Path, _has_headers: bool) -> Result<Vec<Vec<String>>> {
    let content = std::fs::read_to_string(source)?;

    if source.extension().and_then(|e| e.to_str()) == Some("json") {
        // Parse JSON array of arrays
        let data: Vec<Vec<String>> = serde_json::from_str(&content)?;
        Ok(data)
    } else {
        // Parse CSV
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(content.as_bytes());

        let mut rows = Vec::new();
        for result in rdr.records() {
            let record = result?;
            let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            rows.push(row);
        }
        Ok(rows)
    }
}

fn add_table_docx(path: &Path, rows: &[Vec<String>], out: &OutputConfig) -> Result<i32> {
    let data = std::fs::read(path)?;
    let mut doc = docx_rs::read_docx(&data).map_err(|e| {
        OfficeError::FormatError {
            file: path.display().to_string(),
            detail: format!("failed to parse DOCX: {}", e),
        }
    })?;

    // Build table
    let mut table_rows = Vec::new();
    for row_data in rows {
        let mut cells = Vec::new();
        for cell_text in row_data {
            let cell = docx_rs::TableCell::new()
                .add_paragraph(
                    docx_rs::Paragraph::new()
                        .add_run(docx_rs::Run::new().add_text(cell_text))
                );
            cells.push(cell);
        }
        table_rows.push(docx_rs::TableRow::new(cells));
    }

    let table = docx_rs::Table::new(table_rows);
    doc = doc.add_table(table);

    let file = std::fs::File::create(path)?;
    doc.build().pack(file)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "docx".into(),
        message: format!("Table added ({} rows)", rows.len()),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn add_table_xlsx(path: &Path, rows: &[Vec<String>], out: &OutputConfig) -> Result<i32> {
    let mut workbook = umya_spreadsheet::reader::xlsx::read(path).map_err(|e| {
        OfficeError::FormatError {
            file: path.display().to_string(),
            detail: format!("failed to open XLSX: {}", e),
        }
    })?;

    // Add a new sheet for the table
    let sheet_name = format!("Table{}", workbook.get_sheet_count() + 1);
    let sheet = workbook.new_sheet(&sheet_name).map_err(|e| {
        OfficeError::Other(format!("failed to create sheet: {}", e))
    })?;

    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, val) in row.iter().enumerate() {
            sheet.get_cell_mut(((col_idx + 1) as u32, (row_idx + 1) as u32))
                .set_value(val);
        }
    }

    umya_spreadsheet::writer::xlsx::write(&workbook, path).map_err(|e| {
        OfficeError::Io(format!("failed to save: {}", e))
    })?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "xlsx".into(),
        message: format!("Table added to sheet '{}' ({} rows)", sheet_name, rows.len()),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
