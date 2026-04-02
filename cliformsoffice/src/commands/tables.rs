use std::path::Path;
use anyhow::Result;
use crate::cli::TablesArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::{get_backend, PageRange};
use crate::models::TablesResult;
use crate::output::OutputConfig;

pub fn execute(args: TablesArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;

    let page_range = args.page.as_ref().map(|p| PageRange::parse(p)).transpose()?;
    let mut tables = backend.tables(path, page_range.as_ref())?;

    // Apply --headers flag
    if args.headers {
        for table in &mut tables {
            if !table.data.is_empty() {
                table.headers = Some(table.data[0].clone());
                table.data = table.data[1..].to_vec();
                table.rows = table.data.len();
            }
        }
    }

    // Filter by index
    if let Some(idx) = args.index {
        tables = tables.into_iter().filter(|t| t.index == idx).collect();
    }

    if args.csv {
        // Output as CSV
        let mut wtr = csv::Writer::from_writer(std::io::stdout());
        for table in &tables {
            if let Some(ref headers) = table.headers {
                wtr.write_record(headers)?;
            }
            for row in &table.data {
                wtr.write_record(row)?;
            }
        }
        wtr.flush()?;
    } else if out.json {
        let result = TablesResult {
            count: tables.len(),
            tables,
        };
        out.print_json(&result);
    } else {
        for table in &tables {
            out.print_human(&format!("Table {} ({} rows x {} cols):", table.index, table.rows, table.cols));
            if let Some(ref headers) = table.headers {
                out.print_human(&format!("  Headers: {}", headers.join(" | ")));
            }
            for (i, row) in table.data.iter().enumerate() {
                out.print_human(&format!("  [{}] {}", i, row.join(" | ")));
            }
            out.print_human("");
        }
    }
    Ok(0)
}
