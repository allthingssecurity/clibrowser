use anyhow::Result;
use serde::Serialize;

use crate::cli::TablesArgs;
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct TablesResult {
    count: usize,
    tables: Vec<TableItem>,
}

#[derive(Serialize)]
struct TableItem {
    index: usize,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    row_count: usize,
}

pub fn execute(args: TablesArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);
    let mut tables = doc.extract_tables(args.selector.as_deref())?;

    // If --headers flag, use first data row as headers when no th found
    if args.headers {
        for table in &mut tables {
            if table.headers.is_empty() && !table.rows.is_empty() {
                table.headers = table.rows.remove(0);
            }
        }
    }

    // Filter by index
    if let Some(idx) = args.index {
        if idx >= tables.len() {
            return Err(BrowserError::IndexOutOfRange {
                index: idx,
                count: tables.len(),
            }
            .into());
        }
        tables = vec![tables.remove(idx)];
    }

    let items: Vec<TableItem> = tables
        .into_iter()
        .map(|t| TableItem {
            index: t.index,
            row_count: t.rows.len(),
            headers: t.headers,
            rows: t.rows,
        })
        .collect();

    if out.json {
        out.print_json(&TablesResult {
            count: items.len(),
            tables: items,
        });
    } else {
        for table in &items {
            out.print_human(&format!("--- Table {} ({} rows) ---", table.index, table.row_count));
            if !table.headers.is_empty() {
                out.print_human(&table.headers.join(" | "));
                out.print_human(&"-".repeat(table.headers.iter().map(|h| h.len() + 3).sum::<usize>()));
            }
            for row in &table.rows {
                out.print_human(&row.join(" | "));
            }
            out.print_human("");
        }
    }

    Ok(0)
}
