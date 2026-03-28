use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use crate::cli::GetArgs;
use crate::session::Session;

pub struct HttpResponse {
    pub url: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub content_type: Option<String>,
    pub body: String,
    pub redirected_from: Option<String>,
}

/// Build a reqwest client with optional stealth mode for Cloudflare bypass
fn build_client(args: &GetArgs) -> Result<reqwest::Client> {
    let ua = args.user_agent.as_deref().unwrap_or(if args.stealth {
        CHROME_USER_AGENT
    } else {
        "clibrowser/0.1.0"
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(args.timeout))
        .user_agent(ua)
        .redirect(Policy::none())
        // Use rustls for better TLS fingerprint compatibility
        .use_rustls_tls()
        .build()?;

    Ok(client)
}

/// Chrome-like User-Agent string
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Build realistic Chrome browser headers for stealth mode
fn stealth_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
    h.insert("accept-language", HeaderValue::from_static("en-US,en;q=0.9"));
    h.insert("accept-encoding", HeaderValue::from_static("gzip, deflate, br"));
    h.insert("cache-control", HeaderValue::from_static("max-age=0"));
    h.insert("sec-ch-ua", HeaderValue::from_static("\"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\""));
    h.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    h.insert("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\""));
    h.insert("sec-fetch-dest", HeaderValue::from_static("document"));
    h.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
    h.insert("sec-fetch-site", HeaderValue::from_static("none"));
    h.insert("sec-fetch-user", HeaderValue::from_static("?1"));
    h.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
    h.insert("dnt", HeaderValue::from_static("1"));
    h
}

/// Detect if a response is a Cloudflare challenge page
fn is_cloudflare_challenge(status: u16, headers: &HashMap<String, String>, body: &str) -> bool {
    // Cloudflare typically returns 403 or 503 with specific markers
    if status != 403 && status != 503 {
        return false;
    }

    // Check for CF-specific headers
    let has_cf_header = headers.contains_key("cf-ray")
        || headers.contains_key("cf-cache-status")
        || headers.get("server").map(|s| s.contains("cloudflare")).unwrap_or(false);

    if !has_cf_header {
        return false;
    }

    // Check for challenge markers in body
    body.contains("challenge-platform")
        || body.contains("cf-browser-verification")
        || body.contains("cf_clearance")
        || body.contains("Just a moment...")
        || body.contains("Checking your browser")
        || body.contains("Attention Required!")
        || body.contains("cf-turnstile")
        || body.contains("_cf_chl_opt")
}

pub async fn fetch(args: &GetArgs, session: &mut Session) -> Result<HttpResponse> {
    let url_str = resolve_url(&args.url, session)?;
    let original_url = url_str.clone();

    let client = build_client(args)?;

    let method = reqwest::Method::from_str(&args.method.to_uppercase())
        .map_err(|_| anyhow::anyhow!("Invalid HTTP method: {}", args.method))?;

    // Custom headers from CLI
    let mut custom_headers = HeaderMap::new();
    for h in &args.headers {
        if let Some((key, value)) = h.split_once(':') {
            let name = HeaderName::from_str(key.trim())
                .with_context(|| format!("Invalid header name: {}", key.trim()))?;
            let val = HeaderValue::from_str(value.trim())
                .with_context(|| format!("Invalid header value: {}", value.trim()))?;
            custom_headers.insert(name, val);
        }
    }

    let max_redirects = if args.no_follow { 0 } else { args.max_redirects };
    let mut current_url = url_str.clone();
    let mut redirect_count = 0;
    let max_cf_retries = if args.stealth { 2 } else { 0 };
    let mut cf_retry = 0;

    loop {
        let mut req = client.request(method.clone(), &current_url);

        // In stealth mode, add Chrome-like headers first, then overlay custom headers
        if args.stealth {
            req = req.headers(stealth_headers());

            // Add referer for non-first requests (looks more natural)
            if redirect_count > 0 || cf_retry > 0 {
                if let Some(ref prev_url) = session.state.current_url {
                    if let Ok(val) = HeaderValue::from_str(prev_url) {
                        req = req.header("referer", val);
                    }
                }
            }
        }

        req = req.headers(custom_headers.clone());

        // Attach session cookies
        if let Some(cookie_header) = build_cookie_header(session, &current_url) {
            req = req.header("cookie", cookie_header);
        }

        // Only add body on first request (not on redirects)
        if redirect_count == 0 && cf_retry == 0 {
            if let Some(ref data) = args.data {
                req = req
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(data.clone());
            } else if let Some(ref json_data) = args.data_json {
                req = req
                    .header("content-type", "application/json")
                    .body(json_data.clone());
            }
        }

        let response = req.send().await.map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("Request timed out after {}s", args.timeout)
            } else if e.is_connect() {
                anyhow::anyhow!("Connection failed: {}", e)
            } else {
                anyhow::anyhow!("Network error: {}", e)
            }
        })?;

        let status = response.status();

        // Capture set-cookie from this response
        let set_cookies: Vec<String> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .collect();
        update_session_cookies(session, &current_url, &set_cookies);

        // Check for redirect
        if status.is_redirection() && redirect_count < max_redirects {
            if let Some(location) = response.headers().get("location") {
                if let Ok(loc) = location.to_str() {
                    let base = url::Url::parse(&current_url)?;
                    current_url = base.join(loc)?.to_string();
                    redirect_count += 1;
                    continue;
                }
            }
        }

        // Collect response headers
        let mut resp_headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                resp_headers.insert(key.to_string(), v.to_string());
            }
        }

        let body = response.text().await.unwrap_or_default();

        // Cloudflare challenge detection and retry
        if args.stealth && cf_retry < max_cf_retries
            && is_cloudflare_challenge(status.as_u16(), &resp_headers, &body)
        {
            cf_retry += 1;
            eprintln!(
                "clibrowser: Cloudflare challenge detected, retrying ({}/{}) ...",
                cf_retry, max_cf_retries
            );

            // Wait before retry — Cloudflare sometimes allows through after a delay
            // Use jittered delay to look less automated
            let delay_ms = 1000 + (rand::random::<u64>() % 2000);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            continue;
        }

        let final_url = current_url;
        let content_type = resp_headers
            .get("content-type")
            .cloned();

        let redirected_from = if final_url != original_url {
            Some(original_url)
        } else {
            None
        };

        // Update session state
        session.state.current_url = Some(final_url.clone());
        session.state.status_code = Some(status.as_u16());
        session.state.content_type = content_type.clone();
        session.state.response_headers = Some(resp_headers.clone());
        session.state.last_fetched = Some(chrono::Utc::now().to_rfc3339());
        session.save_page(&body)?;

        return Ok(HttpResponse {
            url: final_url,
            status: status.as_u16(),
            headers: resp_headers,
            content_type,
            body,
            redirected_from,
        });
    }
}

fn resolve_url(url: &str, session: &Session) -> Result<String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return Ok(url.to_string());
    }

    if let Some(ref current) = session.state.current_url {
        let base =
            url::Url::parse(current).context("current URL is invalid")?;
        let resolved = base
            .join(url)
            .with_context(|| format!("Cannot resolve '{}' against '{}'", url, current))?;
        Ok(resolved.to_string())
    } else {
        if !url.contains("://") {
            Ok(format!("https://{}", url))
        } else {
            Ok(url.to_string())
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
    pub expires: Option<String>,
}

fn url_matches_cookie(url: &str, cookie: &CookieEntry) -> bool {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let domain = cookie.domain.trim_start_matches('.');
            return host == domain || host.ends_with(&format!(".{}", domain));
        }
    }
    false
}

fn update_session_cookies(session: &mut Session, url: &str, set_cookies: &[String]) {
    if set_cookies.is_empty() {
        return;
    }

    let mut cookies: Vec<CookieEntry> = session
        .cookies_json()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    let parsed_url = url::Url::parse(url).ok();
    let domain = parsed_url
        .as_ref()
        .and_then(|u| u.host_str())
        .unwrap_or("")
        .to_string();

    for set_cookie in set_cookies {
        if let Some(entry) = parse_set_cookie(set_cookie, &domain) {
            cookies.retain(|c| !(c.name == entry.name && c.domain == entry.domain));
            cookies.push(entry);
        }
    }

    if let Ok(json) = serde_json::to_string_pretty(&cookies) {
        session.set_cookies_json(json);
    }
}

fn parse_set_cookie(header: &str, default_domain: &str) -> Option<CookieEntry> {
    let parts: Vec<&str> = header.split(';').collect();
    let name_value = parts.first()?;
    let (name, value) = name_value.split_once('=')?;

    let mut entry = CookieEntry {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        domain: default_domain.to_string(),
        path: "/".to_string(),
        secure: false,
        http_only: false,
        expires: None,
    };

    for part in &parts[1..] {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=') {
            match key.trim().to_lowercase().as_str() {
                "domain" => entry.domain = val.trim().to_string(),
                "path" => entry.path = val.trim().to_string(),
                "expires" => entry.expires = Some(val.trim().to_string()),
                _ => {}
            }
        } else {
            match part.to_lowercase().as_str() {
                "secure" => entry.secure = true,
                "httponly" => entry.http_only = true,
                _ => {}
            }
        }
    }

    Some(entry)
}

fn build_cookie_header(session: &Session, url: &str) -> Option<String> {
    let cookies: Vec<CookieEntry> = session
        .cookies_json()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    let matching: Vec<String> = cookies
        .iter()
        .filter(|c| url_matches_cookie(url, c))
        .map(|c| format!("{}={}", c.name, c.value))
        .collect();

    if matching.is_empty() {
        None
    } else {
        Some(matching.join("; "))
    }
}
