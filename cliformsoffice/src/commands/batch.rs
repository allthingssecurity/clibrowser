use std::path::Path;
use anyhow::Result;
use crate::cli::BatchArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::output::OutputConfig;

pub fn execute(args: BatchArgs, out: &OutputConfig) -> Result<i32> {
    let paths: Vec<_> = glob::glob(&args.glob)
        .map_err(|e| OfficeError::Other(format!("invalid glob: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut results: Vec<serde_json::Value> = Vec::new();

    for entry in &paths {
        let file = entry.display().to_string();
        match process_file(&file, &args.subcommand) {
            Ok(value) => {
                let mut obj = serde_json::Map::new();
                obj.insert("ok".to_string(), serde_json::Value::Bool(true));
                obj.insert("file".to_string(), serde_json::Value::String(file));
                if let serde_json::Value::Object(map) = value {
                    for (k, v) in map { obj.insert(k, v); }
                }
                results.push(serde_json::Value::Object(obj));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "ok": false,
                    "file": file,
                    "error": e.to_string(),
                }));
            }
        }
    }

    if out.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for r in &results {
            if let Some(f) = r.get("file").and_then(|v| v.as_str()) {
                let ok = r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                if ok { out.print_human(&format!("OK: {}", f)); }
                else if let Some(e) = r.get("error").and_then(|v| v.as_str()) {
                    out.print_human(&format!("ERR: {} - {}", f, e));
                }
            }
        }
        out.print_human(&format!("\n{} files processed", results.len()));
    }
    Ok(0)
}

fn process_file(file: &str, subcmd: &str) -> Result<serde_json::Value> {
    let path = Path::new(file);
    if !path.exists() { return Err(OfficeError::FileNotFound(file.to_string()).into()); }

    let kind = detect_format(path, None)?;
    let backend = get_backend(kind)?;

    match subcmd {
        "info" => Ok(serde_json::to_value(backend.info(path)?)?),
        "text" => {
            let text = backend.text(path, None)?;
            Ok(serde_json::json!({"text": text, "length": text.len()}))
        }
        "links" => {
            let links = backend.links(path)?;
            Ok(serde_json::json!({"count": links.len(), "links": links}))
        }
        "tables" => {
            let tables = backend.tables(path, None)?;
            Ok(serde_json::json!({"count": tables.len(), "tables": tables}))
        }
        "comments" => {
            let c = backend.comments(path)?;
            Ok(serde_json::json!({"count": c.len(), "comments": c}))
        }
        "images" => {
            let imgs = backend.images(path, None)?;
            Ok(serde_json::json!({"count": imgs.len(), "images": imgs}))
        }
        "pages" => {
            let p = backend.pages(path)?;
            Ok(serde_json::json!({"count": p.len(), "pages": p}))
        }
        _ => Err(OfficeError::Other(format!("unknown subcommand for batch: {}", subcmd)).into()),
    }
}
