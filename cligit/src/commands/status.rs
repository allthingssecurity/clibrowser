use anyhow::Result;
use git2::StatusOptions;
use crate::cli::StatusArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: StatusArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).renames_head_to_index(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut untracked = Vec::new();
    let mut conflicts = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let s = entry.status();
        if s.is_conflicted() {
            conflicts.push(StatusEntry { path, status: "conflict".into() });
        } else if s.is_index_new() || s.is_index_modified() || s.is_index_deleted() || s.is_index_renamed() || s.is_index_typechange() {
            let st = if s.is_index_new() { "added" } else if s.is_index_deleted() { "deleted" } else if s.is_index_renamed() { "renamed" } else { "modified" };
            staged.push(StatusEntry { path: path.clone(), status: st.into() });
            // File can be both staged and have unstaged changes
            if s.is_wt_modified() || s.is_wt_deleted() {
                let wt = if s.is_wt_deleted() { "deleted" } else { "modified" };
                modified.push(StatusEntry { path, status: wt.into() });
            }
        } else if s.is_wt_new() {
            untracked.push(StatusEntry { path, status: "untracked".into() });
        } else if s.is_wt_modified() || s.is_wt_deleted() {
            let st = if s.is_wt_deleted() { "deleted" } else { "modified" };
            modified.push(StatusEntry { path, status: st.into() });
        }
    }

    let branch = repo.head().ok().and_then(|h| h.shorthand().map(String::from));
    let head_sha = repo.head().ok().and_then(|h| h.target()).map(|o| o.to_string());

    let (upstream, ahead, behind) = match repo.head().ok().and_then(|h| {
        let name = h.shorthand()?.to_string();
        let local = repo.find_branch(&name, git2::BranchType::Local).ok()?;
        let up = local.upstream().ok()?;
        let up_name = up.name().ok()??.to_string();
        let local_oid = h.target()?;
        let up_oid = up.get().target()?;
        let (a, b) = repo.graph_ahead_behind(local_oid, up_oid).ok()?;
        Some((Some(up_name), a, b))
    }) {
        Some((u, a, b)) => (u, a, b),
        None => (None, 0, 0),
    };

    let counts = StatusCounts {
        staged: staged.len(),
        modified: modified.len(),
        untracked: untracked.len(),
        conflicts: conflicts.len(),
    };
    let clean = staged.is_empty() && modified.is_empty() && untracked.is_empty() && conflicts.is_empty();

    let result = StatusResult {
        branch, head_sha, upstream, ahead, behind,
        staged, modified, untracked, conflicts, clean, counts,
    };

    if out.json {
        out.print_json(&result);
    } else {
        if let Some(ref b) = result.branch {
            out.print_human(&format!("On branch {}", b));
        }
        if result.clean {
            out.print_human("nothing to commit, working tree clean");
        } else {
            for e in &result.staged { out.print_human(&format!("  staged: {} ({})", e.path, e.status)); }
            for e in &result.modified { out.print_human(&format!("  modified: {} ({})", e.path, e.status)); }
            for e in &result.untracked { out.print_human(&format!("  untracked: {}", e.path)); }
            for e in &result.conflicts { out.print_human(&format!("  conflict: {}", e.path)); }
        }
    }
    Ok(0)
}
