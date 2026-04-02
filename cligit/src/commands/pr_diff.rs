use anyhow::Result;
use crate::cli::PrDiffArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;
use crate::commands::diff::parse_diff;

pub fn execute(args: PrDiffArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let head_ref = args.head.as_deref().unwrap_or("HEAD");
    let base_obj = repo.revparse_single(&args.base)?;
    let head_obj = repo.revparse_single(head_ref)?;

    let merge_base_oid = repo.merge_base(base_obj.id(), head_obj.id())?;
    let merge_base_tree = repo.find_commit(merge_base_oid)?.tree()?;
    let head_tree = head_obj.peel_to_tree()?;

    let diff = repo.diff_tree_to_tree(Some(&merge_base_tree), Some(&head_tree), None)?;
    let (files, stats) = parse_diff(&diff, false);

    // Count commits from merge_base to head
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    revwalk.push(head_obj.id())?;
    revwalk.hide(merge_base_oid)?;
    let commits = revwalk.count();

    let result = PrDiffResult {
        base: args.base,
        head: head_ref.to_string(),
        merge_base: merge_base_oid.to_string(),
        commits,
        stats,
        files,
    };
    if out.json { out.print_json(&result); }
    else {
        out.print_human(&format!("PR diff: {}...{} (merge-base: {})", result.base, result.head, &result.merge_base[..7]));
        out.print_human(&format!("{} commits, {} files, +{} -{}",
            result.commits, result.stats.files_changed, result.stats.additions, result.stats.deletions));
    }
    Ok(0)
}
