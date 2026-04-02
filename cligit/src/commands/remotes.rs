use anyhow::Result;
use crate::cli::RemotesArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: RemotesArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let remote_names = repo.remotes()?;
    let mut remotes = Vec::new();

    for name in remote_names.iter().flatten() {
        if let Ok(remote) = repo.find_remote(name) {
            remotes.push(RemoteInfo {
                name: name.to_string(),
                fetch_url: remote.url().map(String::from),
                push_url: remote.pushurl().map(String::from).or_else(|| remote.url().map(String::from)),
            });
        }
    }

    let result = RemotesResult { count: remotes.len(), remotes };
    if out.json {
        out.print_json(&result);
    } else {
        for r in &result.remotes {
            out.print_human(&format!("{}\t{}", r.name, r.fetch_url.as_deref().unwrap_or("")));
        }
    }
    Ok(0)
}
