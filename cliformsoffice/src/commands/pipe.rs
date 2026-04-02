use std::io::{self, BufRead};
use std::path::Path;
use anyhow::Result;
use crate::cli::PipeArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::output::OutputConfig;

pub fn execute(args: PipeArgs, _out: &OutputConfig) -> Result<i32> {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let file = line?.trim().to_string();
        if file.is_empty() { continue; }

        let result = process_file(&file, &args.subcommand);
        match result {
            Ok(value) => {
                let mut obj = serde_json::Map::new();
                obj.insert("ok".to_string(), serde_json::Value::Bool(true));
                obj.insert("file".to_string(), serde_json::Value::String(file));
                // Merge result fields
                if let serde_json::Value::Object(map) = value {
                    for (k, v) in map { obj.insert(k, v); }
                }
                println!("{}", serde_json::to_string(&serde_json::Value::Object(obj))?);
            }
            Err(e) => {
                let obj = serde_json::json!({
                    "ok": false,
                    "file": file,
                    "error": e.to_string(),
                });
                println!("{}", serde_json::to_string(&obj)?);
            }
        }
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
        _ => Err(OfficeError::Other(format!("unknown subcommand for pipe: {}", subcmd)).into()),
    }
}
