use std::path::Path;
use anyhow::Result;
use calamine::{Reader, open_workbook_auto, Data};
use crate::cli::SchemaArgs;
use crate::error::OfficeError;
use crate::models::{SchemaColumn, SheetSchema, SchemaResult};
use crate::output::OutputConfig;

pub fn execute(args: SchemaArgs, out: &OutputConfig, _format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file).into()); }

    let mut workbook = open_workbook_auto(path)
        .map_err(|e| OfficeError::Other(format!("cannot open workbook: {}", e)))?;

    let sheet_names: Vec<String> = if let Some(ref s) = args.sheet {
        vec![s.clone()]
    } else {
        workbook.sheet_names().to_vec()
    };

    let mut sheets = Vec::new();
    for name in &sheet_names {
        let range = workbook.worksheet_range(name)
            .map_err(|e| OfficeError::Other(format!("cannot read sheet: {}", e)))?;

        let height = range.height();
        let width = range.width();
        if height == 0 || width == 0 { continue; }

        let sample_rows = height.min(args.sample);
        let mut columns = Vec::new();

        for col in 0..width {
            // Use first row as header
            let header = range.get((0, col))
                .map(|v| format!("{}", v)).unwrap_or_else(|| format!("col_{}", col));

            let mut nulls = 0usize;
            let mut nums: Vec<f64> = Vec::new();
            let mut has_string = false;
            let mut has_bool = false;
            let mut has_date = false;
            let mut sample_val = None;

            for row in 1..sample_rows {
                match range.get((row, col)) {
                    Some(Data::Empty) | None => { nulls += 1; }
                    Some(Data::Int(i)) => { nums.push(*i as f64); }
                    Some(Data::Float(f)) => { nums.push(*f); }
                    Some(Data::Bool(_)) => { has_bool = true; }
                    Some(Data::DateTime(_)) | Some(Data::DateTimeIso(_)) => { has_date = true; }
                    Some(Data::String(s)) => {
                        has_string = true;
                        if sample_val.is_none() && !s.is_empty() { sample_val = Some(s.clone()); }
                    }
                    _ => { has_string = true; }
                }
            }

            let data_type = if !nums.is_empty() && !has_string { "number".into() }
                else if has_date && !has_string { "date".into() }
                else if has_bool && !has_string && nums.is_empty() { "boolean".into() }
                else { "string".into() };

            let (min, max) = if !nums.is_empty() {
                (Some(format!("{}", nums.iter().cloned().fold(f64::INFINITY, f64::min))),
                 Some(format!("{}", nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max))))
            } else { (None, None) };

            columns.push(SchemaColumn {
                name: header, data_type, nulls, sample: sample_val, min, max,
            });
        }

        sheets.push(SheetSchema { sheet: name.clone(), columns, row_count: height.saturating_sub(1) });
    }

    let result = SchemaResult { sheets };
    if out.json {
        out.print_json(&result);
    } else {
        for s in &result.sheets {
            out.print_human(&format!("Sheet: {} ({} rows)", s.sheet, s.row_count));
            for c in &s.columns {
                let extra = c.min.as_ref().map(|mn| format!(" [{} - {}]", mn, c.max.as_deref().unwrap_or("?")))
                    .unwrap_or_default();
                out.print_human(&format!("  {} : {} (nulls: {}){}", c.name, c.data_type, c.nulls, extra));
            }
        }
    }
    Ok(0)
}
