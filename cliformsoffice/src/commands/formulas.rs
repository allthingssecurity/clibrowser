use std::path::Path;
use anyhow::Result;
use crate::cli::FormulasArgs;
use crate::error::OfficeError;
use crate::models::{FormulaInfo, FormulasResult};
use crate::output::OutputConfig;

pub fn execute(args: FormulasArgs, out: &OutputConfig, _format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file).into()); }

    let book = umya_spreadsheet::reader::xlsx::read(&args.file)
        .map_err(|e| OfficeError::Other(format!("xlsx read error: {}", e)))?;

    let mut formulas = Vec::new();

    for sheet in book.get_sheet_collection().iter() {
        let sheet_name_match = args.sheet.as_ref().map(|s| s == sheet.get_name()).unwrap_or(true);
        if !sheet_name_match { continue; }

        for row in 1..=sheet.get_highest_row() {
            for col in 1..=sheet.get_highest_column() {
                let cell = sheet.get_cell((col, row));
                if let Some(cell) = cell {
                    let formula = cell.get_formula().to_string();
                    if !formula.is_empty() {
                        let cell_ref = format!("{}!{}", sheet.get_name(),
                            crate::commands::cells::col_row_to_a1_one((col, row)));
                        let value = Some(cell.get_value().to_string());

                        if let Some(ref target) = args.cell {
                            let target_ref = format!("{}!{}", sheet.get_name(), target.to_uppercase());
                            if cell_ref != target_ref { continue; }
                        }

                        formulas.push(FormulaInfo { cell: cell_ref, formula, value });
                    }
                }
            }
        }
    }

    let result = FormulasResult { count: formulas.len(), formulas };

    if out.json {
        out.print_json(&result);
    } else {
        for f in &result.formulas {
            let val = f.value.as_deref().unwrap_or("");
            out.print_human(&format!("{}: ={} -> {}", f.cell, f.formula, val));
        }
        out.print_human(&format!("\n{} formula(s) found", result.count));
    }
    Ok(0)
}
