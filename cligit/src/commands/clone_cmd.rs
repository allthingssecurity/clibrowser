use anyhow::Result;
use std::process::Command;
use crate::cli::CloneArgs;
use crate::error::GitError;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: CloneArgs, out: &OutputConfig) -> Result<i32> {
    let mut cmd_parts: Vec<String> = vec!["clone".into()];
    if let Some(ref b) = args.branch {
        cmd_parts.push("--branch".into());
        cmd_parts.push(b.clone());
    }
    if let Some(depth) = args.depth {
        cmd_parts.push("--depth".into());
        cmd_parts.push(depth.to_string());
    }
    cmd_parts.push(args.url.clone());
    if let Some(ref dir) = args.directory {
        cmd_parts.push(dir.clone());
    }

    let cmd_refs: Vec<&str> = cmd_parts.iter().map(|s| s.as_str()).collect();
    let output = Command::new("git")
        .args(&cmd_refs)
        .output()
        .map_err(|e| GitError::Io(format!("failed to run git: {}", e)))?;

    if output.status.success() {
        let result = WriteResult {
            message: format!("Cloned {}", args.url),
        };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
        Ok(0)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(GitError::RemoteError(stderr.trim().to_string()).into())
    }
}
