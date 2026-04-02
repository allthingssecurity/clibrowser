use anyhow::Result;
use crate::cli::ResetArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: ResetArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.r#ref)?;

    let mode = match args.mode.as_str() {
        "soft" => git2::ResetType::Soft,
        "hard" => git2::ResetType::Hard,
        _ => git2::ResetType::Mixed,
    };

    repo.reset(&obj, mode, None)?;

    let sha = obj.id().to_string();
    let result = WriteResult {
        message: format!("Reset ({}) to {}", args.mode, &sha[..7.min(sha.len())]),
    };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
