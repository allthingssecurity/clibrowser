use anyhow::Result;
use crate::cli::FilesArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: FilesArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let obj = repo.revparse_single(&args.r#ref)?;
    let tree = obj.peel_to_tree()?;

    let glob_pat = args.pattern.as_ref().and_then(|p| glob::Pattern::new(p).ok());
    let mut files = Vec::new();

    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let path = format!("{}{}", dir, entry.name().unwrap_or(""));
            if let Some(ref pat) = glob_pat {
                if !pat.matches(&path) { return git2::TreeWalkResult::Ok; }
            }
            let size = repo.find_blob(entry.id()).map(|b| b.size() as u64).unwrap_or(0);
            let mode = format!("{:o}", entry.filemode());
            files.push(FileEntry { path, size, mode });
        }
        git2::TreeWalkResult::Ok
    })?;

    let result = FilesResult { count: files.len(), files };
    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("{} files", result.count));
        for f in &result.files {
            out.print_human(&format!("  {} ({}b)", f.path, f.size));
        }
    }
    Ok(0)
}
