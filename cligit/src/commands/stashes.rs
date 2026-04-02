use anyhow::Result;
use crate::cli::StashesArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: StashesArgs, ctx: &mut GitContext, out: &OutputConfig) -> Result<i32> {
    let mut stashes = Vec::new();
    let mut idx = 0usize;
    ctx.repo.stash_foreach(|_index, message, oid| {
        stashes.push(StashInfo {
            index: idx,
            message: message.to_string(),
            sha: oid.to_string(),
        });
        idx += 1;
        true
    })?;

    let result = StashesResult { count: stashes.len(), stashes };
    if out.json {
        out.print_json(&result);
    } else {
        if result.stashes.is_empty() {
            out.print_human("No stashes");
        }
        for s in &result.stashes {
            out.print_human(&format!("stash@{{{}}}: {}", s.index, s.message));
        }
    }
    Ok(0)
}
