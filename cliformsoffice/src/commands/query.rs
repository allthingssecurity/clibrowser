use std::path::Path;
use anyhow::Result;
use calamine::{Reader, open_workbook_auto, Data};
use regex::Regex;
use crate::cli::QueryArgs;
use crate::error::OfficeError;
use crate::models::QueryResult;
use crate::output::OutputConfig;

pub fn execute(args: QueryArgs, out: &OutputConfig, _format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file).into()); }

    let (select_cols, sheet, where_clause, order_by, limit) = parse_sql(&args.sql)?;

    let mut workbook = open_workbook_auto(path)
        .map_err(|e| OfficeError::Other(format!("cannot open workbook: {}", e)))?;

    let sheet_name = sheet.unwrap_or_else(|| workbook.sheet_names().first().unwrap_or(&String::new()).clone());
    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| OfficeError::Other(format!("cannot read sheet: {}", e)))?;

    let height = range.height();
    let width = range.width();
    if height == 0 { return Err(OfficeError::Other("empty sheet".into()).into()); }

    // First row = headers
    let headers: Vec<String> = (0..width)
        .map(|c| range.get((0, c)).map(|v| format!("{}", v)).unwrap_or_default())
        .collect();

    // Determine selected column indices
    let col_indices: Vec<usize> = if select_cols == vec!["*"] {
        (0..width).collect()
    } else {
        select_cols.iter().map(|c| {
            headers.iter().position(|h| h.eq_ignore_ascii_case(c))
                .ok_or_else(|| OfficeError::Other(format!("column '{}' not found", c)))
        }).collect::<std::result::Result<Vec<_>, _>>()?
    };

    let out_cols: Vec<String> = col_indices.iter().map(|&i| headers[i].clone()).collect();

    // Collect rows
    let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
    for r in 1..height {
        let row_data: Vec<String> = (0..width)
            .map(|c| cell_to_string(range.get((r, c))))
            .collect();

        if let Some(ref wc) = where_clause {
            if !eval_where(wc, &headers, &row_data) { continue; }
        }

        let selected: Vec<serde_json::Value> = col_indices.iter()
            .map(|&i| cell_to_json(range.get((r, i))))
            .collect();
        rows.push(selected);
    }

    // Sort
    if let Some(ref ob) = order_by {
        if let Some(idx) = out_cols.iter().position(|h| h.eq_ignore_ascii_case(ob)) {
            rows.sort_by(|a, b| {
                let sa = a.get(idx).and_then(|v| v.as_str()).unwrap_or("");
                let sb = b.get(idx).and_then(|v| v.as_str()).unwrap_or("");
                sa.cmp(sb)
            });
        }
    }

    if let Some(n) = limit { rows.truncate(n); }
    let count = rows.len();

    let result = QueryResult { columns: out_cols, rows, count };

    if out.json {
        out.print_json(&result);
    } else if args.csv {
        println!("{}", result.columns.join(","));
        for row in &result.rows {
            let line: Vec<String> = row.iter().map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }).collect();
            println!("{}", line.join(","));
        }
    } else {
        out.print_human(&format!("Columns: {}", result.columns.join(", ")));
        for row in &result.rows {
            let line: Vec<String> = row.iter().map(|v| format!("{}", v)).collect();
            out.print_human(&line.join(" | "));
        }
        out.print_human(&format!("\n{} rows", count));
    }
    Ok(0)
}

fn parse_sql(sql: &str) -> Result<(Vec<String>, Option<String>, Option<String>, Option<String>, Option<usize>)> {
    let sql = sql.trim();
    let re = Regex::new(r"(?i)^SELECT\s+(.+?)\s+FROM\s+(\S+)(?:\s+WHERE\s+(.+?))?(?:\s+ORDER\s+BY\s+(\S+))?(?:\s+LIMIT\s+(\d+))?$")?;
    let caps = re.captures(sql).ok_or_else(|| OfficeError::Other("cannot parse SQL query".into()))?;

    let cols: Vec<String> = caps[1].split(',').map(|s| s.trim().to_string()).collect();
    let sheet = Some(caps[2].to_string());
    let where_c = caps.get(3).map(|m| m.as_str().to_string());
    let order = caps.get(4).map(|m| m.as_str().to_string());
    let limit = caps.get(5).and_then(|m| m.as_str().parse().ok());

    Ok((cols, sheet, where_c, order, limit))
}

fn eval_where(clause: &str, headers: &[String], row: &[String]) -> bool {
    let re = Regex::new(r"(\w+)\s*(=|!=|>|<|>=|<=|LIKE)\s*'?([^']*)'?").unwrap();
    if let Some(caps) = re.captures(clause) {
        let col = &caps[1];
        let op = &caps[2];
        let val = &caps[3];
        if let Some(idx) = headers.iter().position(|h| h.eq_ignore_ascii_case(col)) {
            let cell = row.get(idx).map(|s| s.as_str()).unwrap_or("");
            return match op {
                "=" => cell == val,
                "!=" => cell != val,
                ">" => cell.parse::<f64>().unwrap_or(0.0) > val.parse::<f64>().unwrap_or(0.0),
                "<" => cell.parse::<f64>().unwrap_or(0.0) < val.parse::<f64>().unwrap_or(0.0),
                ">=" => cell.parse::<f64>().unwrap_or(0.0) >= val.parse::<f64>().unwrap_or(0.0),
                "<=" => cell.parse::<f64>().unwrap_or(0.0) <= val.parse::<f64>().unwrap_or(0.0),
                "LIKE" | "like" => cell.contains(val.trim_matches('%')),
                _ => true,
            };
        }
    }
    true
}

fn cell_to_string(val: Option<&Data>) -> String {
    match val {
        Some(Data::Empty) | None => String::new(),
        Some(v) => format!("{}", v),
    }
}

fn cell_to_json(val: Option<&Data>) -> serde_json::Value {
    match val {
        Some(Data::Int(i)) => serde_json::json!(i),
        Some(Data::Float(f)) => serde_json::json!(f),
        Some(Data::Bool(b)) => serde_json::json!(b),
        Some(Data::String(s)) => serde_json::json!(s),
        Some(Data::Empty) | None => serde_json::Value::Null,
        Some(v) => serde_json::json!(format!("{}", v)),
    }
}
