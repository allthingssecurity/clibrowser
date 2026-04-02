use anyhow::Result;
use crate::cli::MergeArgs;
use crate::error::GitError;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: MergeArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.branch)?;
    let annotated = repo.find_annotated_commit(obj.id())?;
    let (analysis, _pref) = repo.merge_analysis(&[&annotated])?;

    if analysis.is_up_to_date() {
        let result = WriteResult { message: "Already up to date".into() };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
        return Ok(0);
    }

    if analysis.is_fast_forward() && !args.no_ff {
        let target = obj.peel_to_commit()?;
        let refname = repo.head()?.name().unwrap_or("HEAD").to_string();
        repo.reference(&refname, target.id(), true, &format!("Fast-forward to {}", args.branch))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        let result = WriteResult { message: format!("Fast-forwarded to {}", args.branch) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
        return Ok(0);
    }

    // Normal merge
    repo.merge(&[&annotated], None, None)?;

    // Check for conflicts
    let index = repo.index()?;
    if index.has_conflicts() {
        let count = index.conflicts()?.count();
        return Err(GitError::MergeConflict(count).into());
    }

    // Create merge commit
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let sig = repo.signature()?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let merge_commit = obj.peel_to_commit()?;
    let msg = args.message.unwrap_or_else(|| format!("Merge branch '{}'", args.branch));
    repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head_commit, &merge_commit])?;
    repo.cleanup_state()?;

    let result = WriteResult { message: format!("Merged {}", args.branch) };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
