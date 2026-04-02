use std::path::Path;
use std::process::Command;
use crate::error::GitError;

pub fn shell_git(args: &[&str], workdir: &Path) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .map_err(|e| GitError::Io(format!("failed to run git: {}", e)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("Authentication") || stderr.contains("could not read Username") {
            Err(GitError::AuthRequired(stderr.trim().to_string()))
        } else {
            Err(GitError::RemoteError(stderr.trim().to_string()))
        }
    }
}
