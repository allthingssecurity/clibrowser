use anyhow::Result;
use crate::cli::CheckoutArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: CheckoutArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;

    if args.file {
        // Checkout a specific file from a ref
        let ref_name = args.from.as_deref().unwrap_or("HEAD");
        let obj = repo.revparse_single(ref_name)?;
        let tree = obj.peel_to_tree()?;
        let entry = tree.get_path(std::path::Path::new(&args.target))?;
        let blob = repo.find_blob(entry.id())?;
        let dest = ctx.workdir.join(&args.target);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, blob.content())?;
        let result = WriteResult { message: format!("Restored {} from {}", args.target, ref_name) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    } else if args.create {
        // Create and checkout new branch
        let head_commit = repo.head()?.peel_to_commit()?;
        repo.branch(&args.target, &head_commit, false)?;
        let refname = format!("refs/heads/{}", args.target);
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        let result = WriteResult { message: format!("Switched to new branch '{}'", args.target) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    } else {
        // Checkout existing branch
        let refname = format!("refs/heads/{}", args.target);
        // Try as branch first, fall back to detached HEAD
        if repo.find_reference(&refname).is_ok() {
            repo.set_head(&refname)?;
        } else {
            let obj = repo.revparse_single(&args.target)?;
            repo.set_head_detached(obj.id())?;
        }
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        let result = WriteResult { message: format!("Switched to '{}'", args.target) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    }
    Ok(0)
}
