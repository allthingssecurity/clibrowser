use anyhow::Result;
use serde::Serialize;

use crate::cli::GetArgs;
use crate::http;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize)]
struct GetResult {
    url: String,
    status: u16,
    content_type: Option<String>,
    content_length: usize,
    redirected_from: Option<String>,
}

pub async fn execute(args: GetArgs, session: &mut Session, out: &OutputConfig) -> Result<i32> {
    let response = http::fetch(&args, session).await?;

    let result = GetResult {
        url: response.url.clone(),
        status: response.status,
        content_type: response.content_type.clone(),
        content_length: response.body.len(),
        redirected_from: response.redirected_from.clone(),
    };

    if out.json {
        out.print_json(&result);
    } else {
        let status_text = if response.status < 400 { "OK" } else { "ERROR" };
        let ct = response.content_type.as_deref().unwrap_or("unknown");
        let size = format_size(response.body.len());
        out.print_human(&format!(
            "{} {} | {} | {} | {}",
            response.status, status_text, response.url, ct, size
        ));
        if let Some(ref from) = response.redirected_from {
            out.print_human(&format!("  redirected from: {}", from));
        }
    }

    if response.status >= 400 {
        Ok(2)
    } else {
        Ok(0)
    }
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
