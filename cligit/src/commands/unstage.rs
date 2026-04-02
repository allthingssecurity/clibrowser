use anyhow::Result;
use crate::cli::UnstageArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: UnstageArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut index = repo.index()?;

    // Get HEAD tree to reset entries
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    for file in &args.files {
        let path = std::path::Path::new(file);
        if let Some(ref tree) = head_tree {
            if let Ok(entry) = tree.get_path(path) {
                // Reset to HEAD version
                let blob = repo.find_blob(entry.id())?;
                let ie = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0, ino: 0, mode: entry.filemode() as u32,
                    uid: 0, gid: 0,
                    file_size: blob.size() as u32,
                    id: entry.id(),
                    flags: 0, flags_extended: 0,
                    path: file.as_bytes().to_vec(),
                };
                index.add(&ie)?;
            } else {
                // File didn't exist in HEAD, remove from index
                index.remove_path(path)?;
            }
        } else {
            // No HEAD (initial commit), remove from index
            index.remove_path(path)?;
        }
    }
    index.write()?;

    let msg = format!("Unstaged {} file(s)", args.files.len());
    let result = WriteResult { message: msg };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
