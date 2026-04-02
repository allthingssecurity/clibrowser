use anyhow::Result;
use crate::cli::CommitArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: CommitArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut index = repo.index()?;

    if args.all {
        // Stage all modified tracked files
        index.update_all(["*"].iter(), None)?;
        index.write()?;
    }

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let sig = repo.signature()?;

    let parents = match repo.head() {
        Ok(head) => {
            let commit = head.peel_to_commit()?;
            vec![commit]
        }
        Err(_) => vec![], // Initial commit
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, &args.message, &tree, &parent_refs)?;
    let sha = oid.to_string();

    let result = WriteResult {
        message: format!("Created commit {}", &sha[..7.min(sha.len())]),
    };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
