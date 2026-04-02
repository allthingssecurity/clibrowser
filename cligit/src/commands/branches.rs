use anyhow::Result;
use chrono::DateTime;
use crate::cli::BranchesArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: BranchesArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let filter = if args.all {
        None
    } else if args.remote {
        Some(git2::BranchType::Remote)
    } else {
        Some(git2::BranchType::Local)
    };

    let head_ref = repo.head().ok().and_then(|h| h.resolve().ok()).and_then(|h| h.target());
    let current = repo.head().ok().and_then(|h| h.shorthand().map(String::from));

    let mut branches = Vec::new();
    for branch_result in repo.branches(filter)? {
        let (branch, btype) = branch_result?;
        let name = branch.name()?.unwrap_or("").to_string();
        let is_remote = btype == git2::BranchType::Remote;
        let reference = branch.get();
        let sha = reference.target().map(|o| o.to_string()).unwrap_or_default();
        let is_current = !is_remote && head_ref.map(|h| h.to_string()) == Some(sha.clone());

        let (upstream, ahead, behind) = if !is_remote {
            match branch.upstream() {
                Ok(up) => {
                    let up_name = up.name().ok().flatten().map(|s| s.to_string());
                    let (a, b) = if let (Some(local_oid), Some(up_oid)) = (reference.target(), up.get().target()) {
                        repo.graph_ahead_behind(local_oid, up_oid).unwrap_or((0, 0))
                    } else { (0, 0) };
                    (up_name, a, b)
                }
                Err(_) => (None, 0, 0),
            }
        } else { (None, 0, 0) };

        let (last_commit_date, last_commit_message) = reference.target()
            .and_then(|oid| repo.find_commit(oid).ok())
            .map(|c| {
                let date = DateTime::from_timestamp(c.time().seconds(), 0)
                    .map(|d| d.to_rfc3339()).unwrap_or_default();
                let msg = c.message().unwrap_or("").lines().next().unwrap_or("").to_string();
                (Some(date), Some(msg))
            })
            .unwrap_or((None, None));

        branches.push(BranchInfo {
            name, is_current, is_remote, sha, upstream, ahead, behind,
            last_commit_date, last_commit_message,
        });
    }

    let result = BranchesResult {
        current,
        count: branches.len(),
        branches,
    };
    if out.json {
        out.print_json(&result);
    } else {
        for b in &result.branches {
            let prefix = if b.is_current { "* " } else { "  " };
            out.print_human(&format!("{}{} {}", prefix, b.name, &b.sha[..7.min(b.sha.len())]));
        }
    }
    Ok(0)
}
