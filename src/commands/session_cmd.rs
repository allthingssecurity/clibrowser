use anyhow::Result;
use serde::Serialize;

use crate::cli::{SessionAction, SessionArgs};
use crate::output::OutputConfig;
use crate::session::{self, Session};

#[derive(Serialize)]
struct SessionListResult {
    count: usize,
    sessions: Vec<String>,
}

pub fn execute(args: SessionArgs, session: &mut Session, out: &OutputConfig) -> Result<i32> {
    match args.action {
        SessionAction::List => {
            let sessions = session::list_sessions()?;
            if out.json {
                out.print_json(&SessionListResult {
                    count: sessions.len(),
                    sessions,
                });
            } else {
                if sessions.is_empty() {
                    out.print_human("No sessions");
                } else {
                    for s in &sessions {
                        let marker = if *s == session.name { " (active)" } else { "" };
                        out.print_human(&format!("  {}{}", s, marker));
                    }
                }
            }
        }
        SessionAction::Delete { name } => {
            session::delete_session(&name)?;
            if out.json {
                out.print_json(&serde_json::json!({"deleted": true, "name": name}));
            } else {
                out.print_human(&format!("Deleted session: {}", name));
            }
        }
        SessionAction::Clear => {
            session.clear()?;
            if out.json {
                out.print_json(&serde_json::json!({"cleared": true, "session": session.name}));
            } else {
                out.print_human(&format!("Cleared session: {}", session.name));
            }
        }
    }

    Ok(0)
}
