use anyhow::Result;
use crate::cli::CherryPickArgs;
use crate::error::GitError;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: CherryPickArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.commit)?;
    let commit = obj.peel_to_commit()?;
    let sha = commit.id().to_string();

    repo.cherrypick(&commit, None)?;

    let index = repo.index()?;
    if index.has_conflicts() {
        let count = index.conflicts()?.count();
        return Err(GitError::MergeConflict(count).into());
    }

    if !args.no_commit {
        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;
        let sig = repo.signature()?;
        let head = repo.head()?.peel_to_commit()?;
        let msg = commit.message().unwrap_or("cherry-pick");
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&head])?;
        repo.cleanup_state()?;
    }

    let result = WriteResult {
        message: format!("Cherry-picked {}", &sha[..7.min(sha.len())]),
    };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
