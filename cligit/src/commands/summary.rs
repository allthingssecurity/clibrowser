use anyhow::Result;
use chrono::DateTime;
use std::collections::HashSet;
use crate::cli::SummaryArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: SummaryArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;

    let repo_name = ctx.workdir.file_name()
        .and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
    let branch = repo.head().ok().and_then(|h| h.shorthand().map(String::from));
    let head_sha = repo.head().ok().and_then(|h| h.target()).map(|o| o.to_string());

    // Status counts
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut sc = StatusCounts { staged: 0, modified: 0, untracked: 0, conflicts: 0 };
    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_conflicted() { sc.conflicts += 1; }
        else if s.is_index_new() || s.is_index_modified() || s.is_index_deleted() { sc.staged += 1; }
        else if s.is_wt_new() { sc.untracked += 1; }
        else if s.is_wt_modified() || s.is_wt_deleted() { sc.modified += 1; }
    }
    let clean = sc.staged == 0 && sc.modified == 0 && sc.untracked == 0 && sc.conflicts == 0;

    // Recent commits
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    if revwalk.push_head().is_err() {
        // Empty repo
        let result = SummaryResult {
            repo_name, branch, head_sha, clean, status_counts: sc,
            recent_commits: vec![], branch_count: 0, tag_count: 0,
            remote_count: 0, tracked_files: 0, contributors: 0,
        };
        if out.json { out.print_json(&result); }
        else { out.print_human("Empty repository"); }
        return Ok(0);
    }

    let mut recent = Vec::new();
    let mut authors = HashSet::new();
    let mut total_commits = 0usize;
    for oid in revwalk {
        let oid = oid?;
        total_commits += 1;
        let commit = repo.find_commit(oid)?;
        authors.insert(commit.author().email().unwrap_or("").to_string());
        if recent.len() < 5 {
            let sha = oid.to_string();
            let msg = commit.message().unwrap_or("").lines().next().unwrap_or("").to_string();
            let date = DateTime::from_timestamp(commit.time().seconds(), 0)
                .map(|d| d.to_rfc3339()).unwrap_or_default();
            recent.push(CommitInfo {
                short_sha: sha[..7.min(sha.len())].to_string(), sha,
                message: msg, body: None,
                author: AuthorInfo { name: commit.author().name().unwrap_or("").into(), email: commit.author().email().unwrap_or("").into() },
                date, parents: vec![], is_merge: commit.parent_count() > 1,
            });
        }
    }

    let branch_count = repo.branches(Some(git2::BranchType::Local))?.count();
    let tag_count = repo.tag_names(None)?.len();
    let remote_count = repo.remotes()?.len();

    // File count from HEAD tree
    let tracked_files = if let Ok(head) = repo.head() {
        if let Ok(tree) = head.peel_to_tree() {
            let mut count = 0usize;
            tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
                if entry.kind() == Some(git2::ObjectType::Blob) { count += 1; }
                git2::TreeWalkResult::Ok
            }).ok();
            count
        } else { 0 }
    } else { 0 };

    let result = SummaryResult {
        repo_name, branch, head_sha, clean, status_counts: sc,
        recent_commits: recent, branch_count, tag_count, remote_count,
        tracked_files, contributors: authors.len(),
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("Repository: {}", result.repo_name));
        out.print_human(&format!("Branch: {} | Clean: {} | {} files | {} commits by {} authors",
            result.branch.as_deref().unwrap_or("?"), result.clean, result.tracked_files, total_commits, result.contributors));
        out.print_human(&format!("Branches: {} | Tags: {} | Remotes: {}", result.branch_count, result.tag_count, result.remote_count));
    }
    Ok(0)
}
