use anyhow::Result;
use serde::Serialize;

use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct StatusResult {
    session_name: String,
    current_url: Option<String>,
    status_code: Option<u16>,
    content_type: Option<String>,
    last_fetched: Option<String>,
    has_page: bool,
    cookie_count: usize,
}

pub fn execute(session: &Session, out: &OutputConfig) -> Result<i32> {
    let cookie_count: usize = session
        .cookies_json()
        .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
        .map(|v| v.len())
        .unwrap_or(0);

    let result = StatusResult {
        session_name: session.name.clone(),
        current_url: session.state.current_url.clone(),
        status_code: session.state.status_code,
        content_type: session.state.content_type.clone(),
        last_fetched: session.state.last_fetched.clone(),
        has_page: session.page_html().is_some(),
        cookie_count,
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("Session: {}", result.session_name));
        match &result.current_url {
            Some(url) => out.print_human(&format!("URL: {}", url)),
            None => out.print_human("URL: (none)"),
        }
        if let Some(code) = result.status_code {
            out.print_human(&format!("Status: {}", code));
        }
        if let Some(ref ct) = result.content_type {
            out.print_human(&format!("Content-Type: {}", ct));
        }
        if let Some(ref ts) = result.last_fetched {
            out.print_human(&format!("Last fetched: {}", ts));
        }
        out.print_human(&format!("Page cached: {}", result.has_page));
        out.print_human(&format!("Cookies: {}", result.cookie_count));
    }

    Ok(0)
}
