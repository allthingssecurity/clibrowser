use anyhow::Result;
use crate::cli::StageArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: StageArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut index = repo.index()?;

    for file in &args.files {
        let path = std::path::Path::new(file);
        if ctx.workdir.join(path).exists() {
            index.add_path(path)?;
        } else {
            // File was deleted, remove from index
            index.remove_path(path)?;
        }
    }
    index.write()?;

    let msg = format!("Staged {} file(s)", args.files.len());
    let result = WriteResult { message: msg };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
