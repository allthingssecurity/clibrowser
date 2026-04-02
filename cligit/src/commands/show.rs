use anyhow::Result;
use chrono::DateTime;
use crate::cli::ShowArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;
use crate::commands::diff::parse_diff;

pub fn execute(args: ShowArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.r#ref)?;
    let commit = obj.peel_to_commit()?;
    let sha = commit.id().to_string();
    let tree = commit.tree()?;

    let diff = if commit.parent_count() > 0 {
        let parent_tree = commit.parent(0)?.tree()?;
        repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?
    } else {
        repo.diff_tree_to_tree(None, Some(&tree), None)?
    };

    let (files, stats) = parse_diff(&diff, args.stat);

    let msg = commit.message().unwrap_or("").to_string();
    let (first_line, body) = {
        let t = msg.trim();
        if let Some(i) = t.find('\n') {
            (t[..i].trim().to_string(), { let r = t[i+1..].trim().to_string(); if r.is_empty() { None } else { Some(r) } })
        } else {
            (t.to_string(), None)
        }
    };

    let parents: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();
    let date = DateTime::from_timestamp(commit.time().seconds(), 0)
        .map(|d| d.to_rfc3339()).unwrap_or_default();

    let result = ShowResult {
        short_sha: sha[..7.min(sha.len())].to_string(),
        sha,
        message: first_line,
        body,
        author: AuthorInfo {
            name: commit.author().name().unwrap_or("").to_string(),
            email: commit.author().email().unwrap_or("").to_string(),
        },
        committer: AuthorInfo {
            name: commit.committer().name().unwrap_or("").to_string(),
            email: commit.committer().email().unwrap_or("").to_string(),
        },
        date,
        parents,
        stats,
        files,
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("commit {}", result.sha));
        out.print_human(&format!("Author: {} <{}>", result.author.name, result.author.email));
        out.print_human(&format!("Date:   {}", result.date));
        out.print_human(&format!("\n    {}", result.message));
        out.print_human(&format!("\n {} file(s), +{} -{}", result.stats.files_changed, result.stats.additions, result.stats.deletions));
    }
    Ok(0)
}
