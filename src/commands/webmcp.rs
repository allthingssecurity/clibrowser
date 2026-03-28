use anyhow::Result;
use serde::Serialize;

use crate::cli::GetArgs;
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct WebmcpArgs {
    /// URL to discover WebMCP tools from (or use current page)
    pub url: Option<String>,

    /// Filter tools by name
    #[arg(long)]
    pub filter: Option<String>,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(clap::Args)]
pub struct WebmcpCallArgs {
    /// Tool name to invoke
    pub tool: String,

    /// Parameter values as key=value pairs
    pub params: Vec<String>,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(Serialize)]
struct WebmcpDiscoverResult {
    url: Option<String>,
    count: usize,
    tools: Vec<WebmcpTool>,
}

#[derive(Serialize, Clone)]
struct WebmcpTool {
    name: String,
    description: String,
    action: String,
    method: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    autosubmit: bool,
    parameters: Vec<WebmcpParam>,
}

#[derive(Serialize, Clone)]
struct WebmcpParam {
    name: String,
    #[serde(rename = "type")]
    param_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<String>,
}

/// Discover WebMCP tools on a page
pub async fn discover(mut args: WebmcpArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;

    // Fetch page if URL provided
    if let Some(ref url) = args.url {
        let get_args = GetArgs {
            url: url.clone(),
            method: "GET".to_string(),
            headers: vec![],
            data: None,
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: 30,
            user_agent: None,
            stealth: args.stealth,
        };
        crate::http::fetch(&get_args, session).await?;
    }

    let html = session.page_html().ok_or(BrowserError::NoPageLoaded)?;
    let tools = extract_webmcp_tools(&html);

    let mut filtered = tools;
    if let Some(ref f) = args.filter {
        let f = f.to_lowercase();
        filtered.retain(|t| t.name.to_lowercase().contains(&f) || t.description.to_lowercase().contains(&f));
    }

    if out.json {
        out.print_json(&WebmcpDiscoverResult {
            url: session.state.current_url.clone(),
            count: filtered.len(),
            tools: filtered,
        });
    } else {
        if filtered.is_empty() {
            out.print_human("No WebMCP tools found on this page.");
            out.print_human("(WebMCP tools are forms with toolname attribute)");
        } else {
            out.print_human(&format!("WebMCP Tools: {} found\n", filtered.len()));
            for tool in &filtered {
                out.print_human(&format!("  {} — {}", tool.name, tool.description));
                out.print_human(&format!("    {} {}{}", tool.method, tool.action,
                    if tool.autosubmit { " (autosubmit)" } else { "" }));
                for param in &tool.parameters {
                    let req = if param.required { " (required)" } else { "" };
                    let desc = param.description.as_deref().unwrap_or("");
                    let default = param.default_value.as_ref()
                        .map(|v| format!(" [default={}]", v))
                        .unwrap_or_default();
                    out.print_human(&format!("    - {} [{}]{}{} {}", param.name, param.param_type, req, default, desc));
                    if !param.options.is_empty() {
                        out.print_human(&format!("      options: {}", param.options.join(", ")));
                    }
                }
                out.print_human("");
            }
        }
    }

    Ok(0)
}

/// Call a WebMCP tool by name
pub async fn call(mut args: WebmcpCallArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;

    let html = session.page_html().ok_or(BrowserError::NoPageLoaded)?;
    let tools = extract_webmcp_tools(&html);

    let tool = tools.iter()
        .find(|t| t.name == args.tool)
        .ok_or_else(|| anyhow::anyhow!(
            "WebMCP tool '{}' not found. Available: {}",
            args.tool,
            tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
        ))?;

    // Parse params
    let mut param_map = std::collections::HashMap::new();
    for pair in &args.params {
        if let Some((key, value)) = pair.split_once('=') {
            param_map.insert(key.to_string(), value.to_string());
        }
    }

    // Check required params
    for param in &tool.parameters {
        if param.required && !param_map.contains_key(&param.name) {
            return Err(anyhow::anyhow!(
                "Required parameter '{}' not provided for tool '{}'",
                param.name, tool.name
            ));
        }
    }

    // Build form data with defaults
    let mut form_data: Vec<(String, String)> = Vec::new();
    for param in &tool.parameters {
        let value = param_map.get(&param.name)
            .cloned()
            .or_else(|| param.default_value.clone())
            .unwrap_or_default();
        if !value.is_empty() || param.required {
            form_data.push((param.name.clone(), value));
        }
    }

    // Build URL-encoded body
    let encoded: Vec<String> = form_data.iter()
        .map(|(k, v)| format!("{}={}", urlencoding_encode(k), urlencoding_encode(v)))
        .collect();

    let method = tool.method.to_uppercase();
    let action = &tool.action;

    if !out.quiet && !out.json {
        out.print_human(&format!("Calling WebMCP tool: {} ({} {})", tool.name, method, action));
        for (k, v) in &form_data {
            out.print_human(&format!("  {} = {}", k, v));
        }
    }

    if method == "GET" {
        let url = if action.contains('?') {
            format!("{}&{}", action, encoded.join("&"))
        } else {
            format!("{}?{}", action, encoded.join("&"))
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
            stealth: args.stealth,
        };
        super::navigate::execute(get_args, session, out).await
    } else {
        let get_args = GetArgs {
            url: action.clone(),
            method: "POST".to_string(),
            headers: vec![],
            data: Some(encoded.join("&")),
            data_json: None,
            no_follow: false,
            max_redirects: 10,
            timeout: 30,
            user_agent: None,
            stealth: args.stealth,
        };
        super::navigate::execute(get_args, session, out).await
    }
}

fn extract_webmcp_tools(html: &str) -> Vec<WebmcpTool> {
    let doc = Document::parse(html);
    let mut tools = Vec::new();

    // Find forms with toolname attribute
    if let Ok(forms) = doc.select("form[toolname]") {
        for form_el in &forms {
            let name = form_el.attributes.get("toolname")
                .cloned()
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }

            let description = form_el.attributes.get("tooldescription")
                .cloned()
                .unwrap_or_default();
            let action = form_el.attributes.get("action")
                .cloned()
                .unwrap_or_default();
            let method = form_el.attributes.get("method")
                .cloned()
                .unwrap_or_else(|| "GET".to_string())
                .to_uppercase();
            let autosubmit = form_el.attributes.contains_key("toolautosubmit");

            // Parse parameters from form inputs within this form's HTML
            let params = extract_params_from_form_html(&form_el.html);

            tools.push(WebmcpTool {
                name,
                description,
                action,
                method,
                autosubmit,
                parameters: params,
            });
        }
    }

    tools
}

fn extract_params_from_form_html(form_html: &str) -> Vec<WebmcpParam> {
    let doc = Document::parse(form_html);
    let mut params = Vec::new();

    if let Ok(inputs) = doc.select("input, select, textarea") {
        for input in &inputs {
            let name = match input.attributes.get("name") {
                Some(n) if !n.is_empty() => n.clone(),
                _ => continue,
            };

            let input_type = input.attributes.get("type")
                .cloned()
                .unwrap_or_else(|| {
                    match input.tag.as_str() {
                        "select" => "select".to_string(),
                        "textarea" => "textarea".to_string(),
                        _ => "text".to_string(),
                    }
                });

            // Skip submit/hidden types
            if input_type == "submit" || input_type == "hidden" {
                continue;
            }

            let description = input.attributes.get("toolparamdescription").cloned();
            let required = input.attributes.contains_key("required");
            let default_value = input.attributes.get("value")
                .filter(|v| !v.is_empty())
                .cloned();

            // Extract options from select elements
            let options = if input.tag == "select" {
                let opt_doc = Document::parse(&input.html);
                opt_doc.select("option")
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|o| o.attributes.get("value").cloned())
                    .filter(|v| !v.is_empty())
                    .collect()
            } else {
                vec![]
            };

            params.push(WebmcpParam {
                name,
                param_type: input_type,
                description,
                required,
                default_value,
                options,
            });
        }
    }

    params
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}
