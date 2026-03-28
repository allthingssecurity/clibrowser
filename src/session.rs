use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct SessionState {
    pub current_url: Option<String>,
    pub status_code: Option<u16>,
    pub response_headers: Option<HashMap<String, String>>,
    pub content_type: Option<String>,
    pub last_fetched: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FillData {
    pub form_selector: Option<String>,
    pub form_index: Option<usize>,
    pub fields: HashMap<String, String>,
}

pub struct Session {
    pub name: String,
    pub dir: PathBuf,
    pub state: SessionState,
    pub fills: FillData,
    cookies_raw: Option<String>,
}

impl Session {
    pub fn load(name: &str) -> Result<Self> {
        let base = session_base_dir()?;
        let dir = base.join(name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating session dir: {}", dir.display()))?;

        let state: SessionState = match std::fs::read_to_string(dir.join("state.json")) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => SessionState::default(),
        };

        let fills: FillData = match std::fs::read_to_string(dir.join("fills.json")) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => FillData::default(),
        };

        let cookies_raw = std::fs::read_to_string(dir.join("cookies.json")).ok();

        Ok(Session {
            name: name.to_string(),
            dir,
            state,
            fills,
            cookies_raw,
        })
    }

    pub fn save(&self) -> Result<()> {
        let state_json = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(self.dir.join("state.json"), state_json)?;

        if let Some(ref raw) = self.cookies_raw {
            std::fs::write(self.dir.join("cookies.json"), raw)?;
        }

        let fills_json = serde_json::to_string_pretty(&self.fills)?;
        std::fs::write(self.dir.join("fills.json"), fills_json)?;

        Ok(())
    }

    pub fn page_html(&self) -> Option<String> {
        std::fs::read_to_string(self.dir.join("page.html")).ok()
    }

    pub fn save_page(&self, html: &str) -> Result<()> {
        std::fs::write(self.dir.join("page.html"), html)?;
        Ok(())
    }

    pub fn cookies_json(&self) -> Option<&str> {
        self.cookies_raw.as_deref()
    }

    pub fn set_cookies_json(&mut self, json: String) {
        self.cookies_raw = Some(json);
    }

    pub fn clear(&mut self) -> Result<()> {
        self.state = SessionState::default();
        self.fills = FillData::default();
        self.cookies_raw = None;
        // Remove all files in the session dir
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            std::fs::remove_file(entry.path()).ok();
        }
        Ok(())
    }
}

pub fn session_base_dir() -> Result<PathBuf> {
    let base = if let Ok(home) = std::env::var("CLIBROWSER_HOME") {
        PathBuf::from(home)
    } else {
        dirs::home_dir()
            .context("cannot determine home directory")?
            .join(".clibrowser")
    };
    Ok(base.join("sessions"))
}

pub fn list_sessions() -> Result<Vec<String>> {
    let base = session_base_dir()?;
    if !base.exists() {
        return Ok(vec![]);
    }
    let mut sessions = vec![];
    for entry in std::fs::read_dir(&base)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                sessions.push(name.to_string());
            }
        }
    }
    sessions.sort();
    Ok(sessions)
}

pub fn delete_session(name: &str) -> Result<()> {
    let base = session_base_dir()?;
    let dir = base.join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}
