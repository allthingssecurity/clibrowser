use anyhow::Result;
use crate::cli::TagArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: TagArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;

    if args.delete {
        let refname = format!("refs/tags/{}", args.name);
        let mut reference = repo.find_reference(&refname)?;
        reference.delete()?;
        let result = WriteResult { message: format!("Deleted tag {}", args.name) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    } else {
        let target = if let Some(ref r) = args.r#ref {
            repo.revparse_single(r)?
        } else {
            repo.head()?.peel(git2::ObjectType::Commit)?
        };

        if let Some(ref msg) = args.message {
            let sig = repo.signature()?;
            repo.tag(&args.name, &target, &sig, msg, false)?;
        } else {
            repo.tag_lightweight(&args.name, &target, false)?;
        }
        let result = WriteResult { message: format!("Created tag {}", args.name) };
        if out.json { out.print_json(&result); }
        else { out.print_human(&result.message); }
    }
    Ok(0)
}
