use std::path::Path;
use anyhow::Result;
use calamine::{Reader, open_workbook_auto, Data};
use crate::cli::CellsArgs;
use crate::error::OfficeError;
use crate::models::{CellData, CellsResult};
use crate::output::OutputConfig;

pub fn execute(args: CellsArgs, out: &OutputConfig, _format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file.clone()).into()); }

    // Write mode
    if !args.sets.is_empty() {
        return write_cells(&args, out);
    }

    // Read mode
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| OfficeError::Other(format!("cannot open workbook: {}", e)))?;

    let sheet_name = if let Some(ref s) = args.sheet {
        s.clone()
    } else {
        workbook.sheet_names().first()
            .ok_or_else(|| OfficeError::Other("no sheets found".into()))?.clone()
    };

    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| OfficeError::Other(format!("cannot read sheet: {}", e)))?;

    let mut cells = Vec::new();

    if let Some(ref r) = args.range {
        let (start, end) = parse_range(r)?;
        for row in start.1..=end.1 {
            for col in start.0..=end.0 {
                let cell_ref = col_row_to_a1(col, row);
                if let Some(val) = range.get((row, col)) {
                    let (value, ctype) = format_cell_value(val);
                    cells.push(CellData { cell: cell_ref, value, cell_type: ctype });
                } else {
                    cells.push(CellData { cell: cell_ref, value: String::new(), cell_type: "empty".into() });
                }
            }
        }
    } else {
        let (rows, cols) = (range.height(), range.width());
        let limit = rows.min(100);
        for row in 0..limit {
            for col in 0..cols {
                if let Some(val) = range.get((row, col)) {
                    let (value, ctype) = format_cell_value(val);
                    if !value.is_empty() {
                        cells.push(CellData { cell: col_row_to_a1(col, row), value, cell_type: ctype });
                    }
                }
            }
        }
    }

    let result = CellsResult { cells };
    if out.json { out.print_json(&result); } else {
        for c in &result.cells { out.print_human(&format!("{}: {} ({})", c.cell, c.value, c.cell_type)); }
    }
    Ok(0)
}

fn write_cells(args: &CellsArgs, out: &OutputConfig) -> Result<i32> {
    let mut book = umya_spreadsheet::reader::xlsx::read(&args.file)
        .map_err(|e| OfficeError::Other(format!("xlsx read error: {}", e)))?;

    let sheet = if let Some(ref s) = args.sheet {
        book.get_sheet_by_name_mut(s)
            .ok_or_else(|| OfficeError::Other(format!("sheet '{}' not found", s)))?
    } else {
        book.get_sheet_mut(&0usize)
            .ok_or_else(|| OfficeError::Other("no sheets found".into()))?
    };

    for s in &args.sets {
        // Format: A1=value
        if let Some((cell_ref, value)) = s.split_once('=') {
            let (col, row) = parse_a1(cell_ref.trim())?;
            sheet.get_cell_mut(((col + 1) as u32, (row + 1) as u32)).set_value(value.trim());
        }
    }

    umya_spreadsheet::writer::xlsx::write(&book, &args.file)
        .map_err(|e| OfficeError::Other(format!("xlsx write error: {}", e)))?;

    let result = crate::models::WriteResult {
        file: args.file.clone(), format: "xlsx".into(),
        message: format!("{} cell(s) written", args.sets.len()),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn format_cell_value(val: &Data) -> (String, String) {
    match val {
        Data::Int(i) => (i.to_string(), "number".into()),
        Data::Float(f) => (f.to_string(), "number".into()),
        Data::String(s) => (s.clone(), "string".into()),
        Data::Bool(b) => (b.to_string(), "boolean".into()),
        Data::DateTime(d) => (d.to_string(), "date".into()),
        Data::DateTimeIso(s) => (s.clone(), "date".into()),
        Data::DurationIso(s) => (s.clone(), "duration".into()),
        Data::Empty => (String::new(), "empty".into()),
        Data::Error(e) => (format!("{:?}", e), "error".into()),
    }
}

pub fn parse_a1(s: &str) -> Result<(usize, usize)> {
    let s = s.trim().to_uppercase();
    let mut col = 0usize;
    let mut row_str = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphabetic() {
            col = col * 26 + (ch as usize - 'A' as usize + 1);
        } else {
            row_str.push(ch);
        }
    }
    let row: usize = row_str.parse().map_err(|_| OfficeError::Other(format!("invalid cell: {}", s)))?;
    Ok((col.saturating_sub(1), row.saturating_sub(1)))
}

fn parse_range(r: &str) -> Result<((usize, usize), (usize, usize))> {
    if let Some((start, end)) = r.split_once(':') {
        Ok((parse_a1(start)?, parse_a1(end)?))
    } else {
        let p = parse_a1(r)?;
        Ok((p, p))
    }
}

pub fn col_row_to_a1(col: usize, row: usize) -> String {
    let mut c = col;
    let mut letters = String::new();
    loop {
        letters.insert(0, (b'A' + (c % 26) as u8) as char);
        if c < 26 { break; }
        c = c / 26 - 1;
    }
    format!("{}{}", letters, row + 1)
}

/// Convert 1-based (col, row) tuple to A1 notation
pub fn col_row_to_a1_one(pos: (u32, u32)) -> String {
    col_row_to_a1(pos.0 as usize - 1, pos.1 as usize - 1)
}
