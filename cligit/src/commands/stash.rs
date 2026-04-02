use anyhow::Result;
use crate::cli::StashArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: StashArgs, ctx: &mut GitContext, out: &OutputConfig) -> Result<i32> {
    match args.action.as_str() {
        "push" | "save" => {
            let sig = ctx.repo.signature()?;
            let msg = args.message.as_deref().unwrap_or("");
            let oid = ctx.repo.stash_save(&sig, msg, None)?;
            let result = WriteResult { message: format!("Saved stash {}", &oid.to_string()[..7]) };
            if out.json { out.print_json(&result); }
            else { out.print_human(&result.message); }
        }
        "pop" => {
            let idx = args.index.unwrap_or(0);
            ctx.repo.stash_pop(idx, None)?;
            let result = WriteResult { message: format!("Popped stash@{{{}}}", idx) };
            if out.json { out.print_json(&result); }
            else { out.print_human(&result.message); }
        }
        "apply" => {
            let idx = args.index.unwrap_or(0);
            ctx.repo.stash_apply(idx, None)?;
            let result = WriteResult { message: format!("Applied stash@{{{}}}", idx) };
            if out.json { out.print_json(&result); }
            else { out.print_human(&result.message); }
        }
        "drop" => {
            let idx = args.index.unwrap_or(0);
            ctx.repo.stash_drop(idx)?;
            let result = WriteResult { message: format!("Dropped stash@{{{}}}", idx) };
            if out.json { out.print_json(&result); }
            else { out.print_human(&result.message); }
        }
        "list" => {
            let mut stashes = Vec::new();
            let mut i = 0usize;
            ctx.repo.stash_foreach(|_index, message, oid| {
                stashes.push(StashInfo { index: i, message: message.to_string(), sha: oid.to_string() });
                i += 1;
                true
            })?;
            let result = StashesResult { count: stashes.len(), stashes };
            if out.json { out.print_json(&result); }
            else { for s in &result.stashes { out.print_human(&format!("stash@{{{}}}: {}", s.index, s.message)); } }
        }
        other => {
            anyhow::bail!("Unknown stash action: {}. Use push, pop, apply, drop, or list.", other);
        }
    }
    Ok(0)
}
