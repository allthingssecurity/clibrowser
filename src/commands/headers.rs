use anyhow::Result;
use serde::Serialize;

use crate::cli::HeadersArgs;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct HeadersResult {
    count: usize,
    headers: std::collections::HashMap<String, String>,
}

#[derive(Serialize)]
struct SingleHeaderResult {
    name: String,
    value: Option<String>,
}

pub fn execute(args: HeadersArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let headers = session
        .state
        .response_headers
        .as_ref()
        .ok_or(BrowserError::NoPageLoaded)?;

    if let Some(ref name) = args.name {
        let lower = name.to_lowercase();
        let value = headers.get(&lower).cloned();

        if out.json {
            out.print_json(&SingleHeaderResult {
                name: name.clone(),
                value: value.clone(),
            });
        } else {
            match value {
                Some(v) => out.print_human(&v),
                None => out.print_human(&format!("(header '{}' not found)", name)),
            }
        }
    } else {
        if out.json {
            out.print_json(&HeadersResult {
                count: headers.len(),
                headers: headers.clone(),
            });
        } else {
            let mut sorted: Vec<_> = headers.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            for (key, value) in sorted {
                out.print_human(&format!("{}: {}", key, value));
            }
        }
    }

    Ok(0)
}
