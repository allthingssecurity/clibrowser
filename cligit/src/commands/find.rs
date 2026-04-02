use anyhow::Result;
use glob::Pattern;
use crate::cli::FindArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: FindArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.r#ref)?;
    let tree = obj.peel_to_tree()?;
    let pat = Pattern::new(&args.pattern)?;

    let mut files = Vec::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let path = format!("{}{}", dir, entry.name().unwrap_or(""));
            if pat.matches(&path) {
                files.push(path);
            }
        }
        git2::TreeWalkResult::Ok
    })?;

    let result = FindResult { pattern: args.pattern, count: files.len(), files };
    if out.json {
        out.print_json(&result);
    } else {
        for f in &result.files {
            out.print_human(f);
        }
    }
    Ok(0)
}
