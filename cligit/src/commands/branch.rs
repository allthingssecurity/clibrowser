use anyhow::Result;
use crate::cli::BranchArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: BranchArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;

    if args.delete {
        let mut branch = repo.find_branch(&args.name, git2::BranchType::Local)?;
        branch.delete()?;
        let result = WriteResult { message: format!("Deleted branch {}", args.name) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    } else {
        let target_commit = if let Some(ref from) = args.from {
            repo.revparse_single(from)?.peel_to_commit()?
        } else {
            repo.head()?.peel_to_commit()?
        };
        repo.branch(&args.name, &target_commit, false)?;
        let result = WriteResult { message: format!("Created branch {}", args.name) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    }
    Ok(0)
}
