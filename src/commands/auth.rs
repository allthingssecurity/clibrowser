use anyhow::Result;
use serde::Serialize;
use std::io::Write;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

use crate::cli::GetArgs;
use crate::output::OutputConfig;
use crate::session::Session;

#[derive(clap::Args)]
pub struct AuthArgs {
    /// URL of the login page to open in browser
    pub url: String,

    /// Local port for the callback listener
    #[arg(long, default_value = "19876")]
    pub port: u16,

    /// Timeout in seconds to wait for login completion
    #[arg(long, default_value = "120")]
    pub timeout: u64,

    /// Don't open browser automatically (just print the URL)
    #[arg(long)]
    pub no_open: bool,

    /// Stealth mode (passed from global flag)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(Serialize)]
struct AuthResult {
    logged_in: bool,
    cookies_captured: usize,
    url: String,
}

pub async fn execute(mut args: AuthArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    args.stealth = stealth;

    // Step 1: Fetch the login page first to get initial cookies (CSRF etc.)
    if !out.quiet {
        eprintln!("Step 1: Fetching login page for initial cookies...");
    }
    let get_args = GetArgs {
        url: args.url.clone(),
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

    // Step 2: Start local callback server
    let addr = format!("127.0.0.1:{}", args.port);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| anyhow::anyhow!("Cannot bind to {}: {}. Try --port <other>", addr, e))?;

    if !out.quiet {
        eprintln!("Step 2: Callback server listening on http://{}", addr);
    }

    // Step 3: Build the instruction page URL
    let login_url = args.url.clone();
    let callback_url = format!("http://127.0.0.1:{}/callback", args.port);

    if !out.quiet {
        eprintln!("Step 3: Opening browser for login...");
        eprintln!();
        eprintln!("  Login URL: {}", login_url);
        eprintln!("  After login, paste the final URL at the prompt below.");
        eprintln!("  Or the page will auto-redirect if possible.");
        eprintln!();
    }

    // Open browser
    if !args.no_open {
        open_browser(&login_url);
    } else {
        eprintln!("Open this URL in your browser:");
        eprintln!("  {}", login_url);
    }

    // Step 4: Wait for user to complete login, then ask for the final URL
    // Two modes:
    //   a) User pastes the URL they landed on after login
    //   b) Callback server catches a redirect (for sites that support custom redirect_uri)

    // Start a race between callback listener and stdin prompt
    let timeout_duration = std::time::Duration::from_secs(args.timeout);

    eprintln!("Waiting for login ({}s timeout)...", args.timeout);
    eprintln!("After logging in, either:");
    eprintln!("  1. Copy the URL from your browser address bar and paste it here");
    eprintln!("  2. Or just press Enter if you see the site's dashboard");
    eprintln!();

    // Wait for either callback or user input
    let result = tokio::select! {
        // Listen for callback
        accepted = listener.accept() => {
            match accepted {
                Ok((mut stream, _)) => {
                    let mut buf = vec![0u8; 4096];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();

                    // Extract the path from the HTTP request
                    let path = request.lines().next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/");

                    // Send a nice response back to the browser
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                        <html><body style='font-family:system-ui;text-align:center;padding:60px'>\
                        <h1>Login captured!</h1>\
                        <p>You can close this tab and return to the terminal.</p>\
                        </body></html>"
                    );
                    let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;

                    Some(format!("callback:{}", path))
                }
                Err(_) => None,
            }
        }
        // Wait for stdin (user pastes URL)
        line = async {
            tokio::task::spawn_blocking(|| {
                eprint!("Paste URL (or Enter): ");
                std::io::stderr().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                input.trim().to_string()
            }).await.ok()
        } => {
            line.map(|s| format!("stdin:{}", s))
        }
        // Timeout
        _ = tokio::time::sleep(timeout_duration) => {
            None
        }
    };

    // Step 5: Now fetch the authenticated page to capture session cookies
    if !out.quiet {
        eprintln!("Step 5: Capturing session cookies...");
    }

    let target_url = match result {
        Some(ref r) if r.starts_with("stdin:") => {
            let url = r.strip_prefix("stdin:").unwrap();
            if url.is_empty() || !url.starts_with("http") {
                // User just pressed Enter — re-fetch the original site root
                let base = url::Url::parse(&args.url)?;
                format!("{}://{}/", base.scheme(), base.host_str().unwrap_or(""))
            } else {
                url.to_string()
            }
        }
        Some(ref r) if r.starts_with("callback:") => {
            // Got a callback — extract any tokens from the path
            let path = r.strip_prefix("callback:").unwrap();
            if !out.quiet {
                eprintln!("  Received callback: {}", path);
            }
            // Re-fetch the site root with whatever cookies we have
            let base = url::Url::parse(&args.url)?;
            format!("{}://{}/", base.scheme(), base.host_str().unwrap_or(""))
        }
        _ => {
            eprintln!("Timeout waiting for login.");
            return Ok(1);
        }
    };

    // Fetch the target URL — if login succeeded, the site will set session cookies
    let get_args = GetArgs {
        url: target_url.clone(),
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

    let response = crate::http::fetch(&get_args, session).await?;

    // Count cookies
    let cookie_count: usize = session
        .cookies_json()
        .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
        .map(|v| v.len())
        .unwrap_or(0);

    // Check if we're actually logged in (look for common indicators)
    let logged_in = cookie_count > 1 || {
        // Check for sessionid-like cookies
        let cookies_str = session.cookies_json().unwrap_or("[]");
        cookies_str.contains("session") || cookies_str.contains("auth") || cookies_str.contains("token")
    };

    if out.json {
        out.print_json(&AuthResult {
            logged_in,
            cookies_captured: cookie_count,
            url: response.url.clone(),
        });
    } else {
        if logged_in {
            out.print_human(&format!("Login successful! {} cookies captured.", cookie_count));
            out.print_human(&format!("Current URL: {}", response.url));
            out.print_human("You can now use other clibrowser commands with this session.");
        } else {
            out.print_human(&format!("Login may not have succeeded. {} cookies captured.", cookie_count));
            out.print_human("Try browsing an authenticated page to check:");
            out.print_human(&format!("  clibrowser --session {} get \"{}\"", session.name, target_url));
        }
    }

    Ok(0)
}

/// Import cookies directly from a browser cookie string or JSON
pub fn import_cookies(session: &mut Session, cookies_input: &str, domain: &str, out: &OutputConfig) -> Result<i32> {
    let mut imported = 0;

    // Try parsing as JSON array first
    if let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(cookies_input) {
        let mut cookies: Vec<crate::http::CookieEntry> = session
            .cookies_json()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        for entry in &entries {
            if let (Some(name), Some(value)) = (entry["name"].as_str(), entry["value"].as_str()) {
                let d = entry["domain"].as_str().unwrap_or(domain).to_string();
                cookies.retain(|c| !(c.name == name && c.domain == d));
                cookies.push(crate::http::CookieEntry {
                    name: name.to_string(),
                    value: value.to_string(),
                    domain: d,
                    path: entry["path"].as_str().unwrap_or("/").to_string(),
                    secure: entry["secure"].as_bool().unwrap_or(false),
                    http_only: entry["httpOnly"].as_bool().unwrap_or(false),
                    expires: entry["expires"].as_str().map(|s| s.to_string()),
                });
                imported += 1;
            }
        }

        session.set_cookies_json(serde_json::to_string_pretty(&cookies)?);
    } else {
        // Parse as "name=value; name2=value2" cookie header string
        let mut cookies: Vec<crate::http::CookieEntry> = session
            .cookies_json()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        for pair in cookies_input.split(';') {
            let pair = pair.trim();
            if let Some((name, value)) = pair.split_once('=') {
                let name = name.trim().to_string();
                let value = value.trim().to_string();
                cookies.retain(|c| !(c.name == name && c.domain == domain));
                cookies.push(crate::http::CookieEntry {
                    name,
                    value,
                    domain: domain.to_string(),
                    path: "/".to_string(),
                    secure: false,
                    http_only: false,
                    expires: None,
                });
                imported += 1;
            }
        }

        session.set_cookies_json(serde_json::to_string_pretty(&cookies)?);
    }

    if out.json {
        out.print_json(&serde_json::json!({
            "imported": imported,
            "domain": domain,
        }));
    } else {
        out.print_human(&format!("Imported {} cookies for {}", imported, domain));
    }

    Ok(0)
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
    }
}
