use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;
use regex::Regex;
use crate::cli::ExtractArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::ExtractResult;
use crate::output::OutputConfig;

pub fn execute(args: ExtractArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() { return Err(OfficeError::FileNotFound(args.file).into()); }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;
    let text = backend.text(path, None)?;

    if args.all {
        let mut all_matches: Vec<serde_json::Value> = Vec::new();
        for pat in &args.patterns {
            let (regex_str, names) = pattern_to_regex(pat);
            let re = Regex::new(&regex_str)?;
            for caps in re.captures_iter(&text) {
                let mut map = serde_json::Map::new();
                for name in &names {
                    if let Some(m) = caps.name(name) {
                        map.insert(name.clone(), serde_json::Value::String(m.as_str().to_string()));
                    }
                }
                all_matches.push(serde_json::Value::Object(map));
            }
        }
        let mut fields = HashMap::new();
        fields.insert("matches".to_string(), serde_json::Value::Array(all_matches));
        let result = ExtractResult { fields };
        if out.json { out.print_json(&result); } else {
            out.print_human(&serde_json::to_string_pretty(&result.fields)?);
        }
    } else {
        let mut fields = HashMap::new();
        for pat in &args.patterns {
            let (regex_str, names) = pattern_to_regex(pat);
            let re = Regex::new(&regex_str)?;
            if let Some(caps) = re.captures(&text) {
                for name in &names {
                    if let Some(m) = caps.name(name) {
                        fields.insert(name.clone(), serde_json::Value::String(m.as_str().to_string()));
                    }
                }
            }
        }
        let result = ExtractResult { fields };
        if out.json { out.print_json(&result); } else {
            for (k, v) in &result.fields { out.print_human(&format!("{}: {}", k, v)); }
        }
    }
    Ok(0)
}

/// Convert "Invoice #: {id}" → regex with named group, return (regex_string, group_names)
fn pattern_to_regex(pattern: &str) -> (String, Vec<String>) {
    let mut names = Vec::new();
    let mut regex_str = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
            names.push(name.clone());
            regex_str.push_str(&format!("(?P<{}>.+?)", name));
        } else {
            // Escape regex special chars in literal parts
            if "\\^$.|?*+()[]".contains(ch) {
                regex_str.push('\\');
            }
            regex_str.push(ch);
        }
    }
    (regex_str, names)
}
