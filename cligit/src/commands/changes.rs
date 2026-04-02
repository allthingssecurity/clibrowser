use anyhow::Result;
use std::collections::HashSet;
use crate::cli::ChangesArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;
use crate::commands::diff::parse_diff;

pub fn execute(args: ChangesArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let to_ref = args.to.as_deref().unwrap_or("HEAD");
    let from_obj = repo.revparse_single(&args.from)?;
    let to_obj = repo.revparse_single(to_ref)?;

    // Count commits between from..to
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    revwalk.push(to_obj.id())?;
    revwalk.hide(from_obj.id())?;
    let mut commit_count = 0;
    let mut authors = HashSet::new();
    for oid in revwalk {
        let oid = oid?;
        commit_count += 1;
        if let Ok(c) = repo.find_commit(oid) {
            authors.insert(c.author().name().unwrap_or("").to_string());
        }
    }

    let from_tree = from_obj.peel_to_tree()?;
    let to_tree = to_obj.peel_to_tree()?;
    let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;
    let (files, stats) = parse_diff(&diff, false);

    let result = ChangesResult {
        from_ref: args.from,
        to_ref: to_ref.to_string(),
        commit_count,
        authors: authors.into_iter().collect(),
        stats,
        files,
    };
    if out.json { out.print_json(&result); }
    else {
        out.print_human(&format!("{}..{}: {} commits, {} files changed, +{} -{}",
            result.from_ref, result.to_ref, result.commit_count,
            result.stats.files_changed, result.stats.additions, result.stats.deletions));
    }
    Ok(0)
}
