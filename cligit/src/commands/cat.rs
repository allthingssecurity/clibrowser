use anyhow::Result;
use crate::cli::CatArgs;
use crate::error::GitError;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: CatArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.r#ref)?;
    let tree = obj.peel_to_tree()?;
    let entry = tree.get_path(std::path::Path::new(&args.file))
        .map_err(|_| GitError::FileNotFound(args.file.clone(), args.r#ref.clone()))?;
    let blob = repo.find_blob(entry.id())?;
    let content = std::str::from_utf8(blob.content())
        .unwrap_or("[binary content]").to_string();
    let lines = content.lines().count();

    let result = CatResult {
        file: args.file.clone(),
        size: blob.size(),
        lines,
        content,
    };
    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&result.content);
    }
    Ok(0)
}
