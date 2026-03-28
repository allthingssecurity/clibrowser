use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;

use crate::cli::{FillArgs, FormsArgs, GetArgs, SubmitArgs};
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct FormsResult {
    count: usize,
    forms: Vec<FormItem>,
}

#[derive(Serialize)]
struct FormItem {
    index: usize,
    action: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    fields: Vec<crate::dom::FormField>,
}

#[derive(Serialize)]
struct FillResult {
    form_selector: Option<String>,
    form_index: Option<usize>,
    fields_set: usize,
}

pub fn execute(args: FormsArgs, session: &Session, out: &OutputConfig) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);
    let mut forms = doc.extract_forms()?;

    // Filter by selector
    if let Some(ref sel) = args.selector {
        let elements = doc.select(sel)?;
        let form_indices: Vec<usize> = elements.iter().map(|e| e.index).collect();
        forms.retain(|f| form_indices.contains(&f.index));
    }

    // Filter by index
    if let Some(idx) = args.index {
        if idx >= forms.len() {
            return Err(BrowserError::NoFormAtIndex(idx).into());
        }
        forms = vec![forms.remove(idx)];
    }

    let items: Vec<FormItem> = forms
        .into_iter()
        .map(|f| FormItem {
            index: f.index,
            action: f.action,
            method: f.method,
            id: f.id,
            name: f.name,
            fields: f.fields,
        })
        .collect();

    if out.json {
        out.print_json(&FormsResult {
            count: items.len(),
            forms: items,
        });
    } else {
        for form in &items {
            let id_str = form
                .id
                .as_ref()
                .map(|id| format!(" id=\"{}\"", id))
                .unwrap_or_default();
            out.print_human(&format!(
                "[{}] <form{}> action=\"{}\" method={}",
                form.index, id_str, form.action, form.method
            ));
            for field in &form.fields {
                let req = if field.required { " (required)" } else { "" };
                out.print_human(&format!(
                    "  {} [{}] = \"{}\"{}", field.name, field.field_type, field.value, req
                ));
            }
            out.print_human("");
        }
    }

    Ok(0)
}

pub fn fill(args: FillArgs, session: &mut Session, out: &OutputConfig) -> Result<i32> {
    let mut fields = HashMap::new();
    for pair in &args.fields {
        if let Some((key, value)) = pair.split_once('=') {
            fields.insert(key.to_string(), value.to_string());
        } else {
            return Err(anyhow::anyhow!(
                "Invalid field format: '{}'. Expected 'name=value'.",
                pair
            ));
        }
    }

    session.fills.form_selector = Some(args.selector.clone());
    session.fills.form_index = args.index;
    session.fills.fields = fields.clone();

    if out.json {
        out.print_json(&FillResult {
            form_selector: Some(args.selector),
            form_index: args.index,
            fields_set: fields.len(),
        });
    } else {
        out.print_human(&format!("Filled {} field(s)", fields.len()));
    }

    Ok(0)
}

pub async fn submit(args: SubmitArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);
    let forms = doc.extract_forms()?;

    // Find the target form
    let form = if let Some(idx) = args.index.or(session.fills.form_index) {
        forms
            .into_iter()
            .nth(idx)
            .ok_or(BrowserError::NoFormAtIndex(idx))?
    } else if let Some(ref sel) = args.selector.as_ref().or(session.fills.form_selector.as_ref()) {
        let elements = doc.select(sel)?;
        if elements.is_empty() {
            return Err(BrowserError::NoFormFound(sel.to_string()).into());
        }
        let target_idx = elements[0].index;
        let forms = doc.extract_forms()?;
        forms
            .into_iter()
            .find(|f| f.index == target_idx)
            .ok_or_else(|| BrowserError::NoFormFound(sel.to_string()))?
    } else {
        // Default to first form
        let forms = doc.extract_forms()?;
        forms
            .into_iter()
            .next()
            .ok_or(BrowserError::NoFormFound("(default)".to_string()))?
    };

    // Build form data by merging defaults with fills
    let mut form_data: Vec<(String, String)> = Vec::new();
    for field in &form.fields {
        if field.field_type == "submit" {
            // Only include submit button value if it matches --button
            if let Some(ref btn) = args.button {
                if field.name == *btn {
                    form_data.push((field.name.clone(), field.value.clone()));
                }
            }
            continue;
        }

        let value = session
            .fills
            .fields
            .get(&field.name)
            .cloned()
            .unwrap_or_else(|| field.value.clone());
        form_data.push((field.name.clone(), value));
    }

    // Build the URL
    let action = &form.action;
    let method = form.method.to_uppercase();

    // Clear fills after submission
    session.fills = crate::session::FillData::default();

    if method == "GET" {
        // Append as query parameters
        let query: Vec<String> = form_data
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding_encode(k), urlencoding_encode(v)))
            .collect();
        let url = if action.contains('?') {
            format!("{}&{}", action, query.join("&"))
        } else {
            format!("{}?{}", action, query.join("&"))
        };

        let get_args = GetArgs {
            url,
            method: "GET".to_string(),
            headers: vec![],
            data: None,
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: 30,
            user_agent: None,
            stealth,
        };
        super::navigate::execute(get_args, session, out).await
    } else {
        // POST with form-encoded body
        let body: Vec<String> = form_data
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding_encode(k), urlencoding_encode(v)))
            .collect();

        let get_args = GetArgs {
            url: action.clone(),
            method: "POST".to_string(),
            headers: vec![],
            data: Some(body.join("&")),
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: 30,
            user_agent: None,
            stealth,
        };
        super::navigate::execute(get_args, session, out).await
    }
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
