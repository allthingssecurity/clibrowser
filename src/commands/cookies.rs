use anyhow::Result;
use serde::Serialize;

use crate::cli::{CookieAction, CookiesArgs};
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(Serialize, serde::Deserialize, Clone)]
struct CookieEntry {
    name: String,
    value: String,
    domain: String,
    path: String,
    #[serde(default)]
    secure: bool,
    #[serde(default)]
    http_only: bool,
    expires: Option<String>,
}

#[derive(Serialize)]
struct CookiesResult {
    count: usize,
    cookies: Vec<CookieEntry>,
}

pub fn execute(args: CookiesArgs, session: &mut Session, out: &OutputConfig) -> Result<i32> {
    match args.action {
        Some(CookieAction::Set {
            name,
            value,
            domain,
            path,
        }) => {
            let current_domain = session
                .state
                .current_url
                .as_ref()
                .and_then(|u| url::Url::parse(u).ok())
                .and_then(|u| u.host_str().map(|s| s.to_string()))
                .unwrap_or_default();

            let entry = CookieEntry {
                name: name.clone(),
                value: value.clone(),
                domain: domain.unwrap_or(current_domain),
                path: path.unwrap_or_else(|| "/".to_string()),
                secure: false,
                http_only: false,
                expires: None,
            };

            let mut cookies: Vec<CookieEntry> = session
                .cookies_json()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default();

            cookies.retain(|c| !(c.name == entry.name && c.domain == entry.domain));
            cookies.push(entry);

            session.set_cookies_json(serde_json::to_string_pretty(&cookies)?);

            if out.json {
                out.print_json(&serde_json::json!({"set": true, "name": name, "value": value}));
            } else {
                out.print_human(&format!("Cookie set: {}={}", name, value));
            }
        }
        Some(CookieAction::Clear { domain }) => {
            if let Some(ref d) = domain {
                let mut cookies: Vec<CookieEntry> = session
                    .cookies_json()
                    .and_then(|j| serde_json::from_str(j).ok())
                    .unwrap_or_default();
                let before = cookies.len();
                cookies.retain(|c| !c.domain.ends_with(d));
                let removed = before - cookies.len();
                session.set_cookies_json(serde_json::to_string_pretty(&cookies)?);

                if out.json {
                    out.print_json(&serde_json::json!({"cleared": true, "domain": d, "removed": removed}));
                } else {
                    out.print_human(&format!("Cleared {} cookie(s) for {}", removed, d));
                }
            } else {
                session.set_cookies_json("[]".to_string());

                if out.json {
                    out.print_json(&serde_json::json!({"cleared": true, "all": true}));
                } else {
                    out.print_human("All cookies cleared");
                }
            }
        }
        None => {
            // List cookies
            let cookies: Vec<CookieEntry> = session
                .cookies_json()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default();

            let filtered = if args.all {
                cookies
            } else {
                let current_domain = session
                    .state
                    .current_url
                    .as_ref()
                    .and_then(|u| url::Url::parse(u).ok())
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                cookies
                    .into_iter()
                    .filter(|c| {
                        c.domain == current_domain
                            || current_domain.ends_with(&format!(".{}", c.domain.trim_start_matches('.')))
                            || c.domain.trim_start_matches('.') == current_domain
                    })
                    .collect()
            };

            if out.json {
                out.print_json(&CookiesResult {
                    count: filtered.len(),
                    cookies: filtered,
                });
            } else {
                if filtered.is_empty() {
                    out.print_human("No cookies");
                } else {
                    for c in &filtered {
                        out.print_human(&format!(
                            "{}={} (domain={}, path={})",
                            c.name, c.value, c.domain, c.path
                        ));
                    }
                }
            }
        }
    }

    Ok(0)
}
