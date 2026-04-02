use anyhow::Result;
use crate::cli::ConflictsArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: ConflictsArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let index = repo.index()?;
    let mut conflict_entries = Vec::new();

    if index.has_conflicts() {
        for conflict in index.conflicts()? {
            let conflict = conflict?;
            let path = conflict.our
                .as_ref()
                .or(conflict.their.as_ref())
                .and_then(|e| std::str::from_utf8(&e.path).ok())
                .unwrap_or("")
                .to_string();
            let ours_ref = conflict.our.as_ref().map(|e| e.id.to_string());
            let theirs_ref = conflict.their.as_ref().map(|e| e.id.to_string());
            conflict_entries.push(ConflictEntry { path, ours_ref, theirs_ref });
        }
    }

    let result = ConflictsResult {
        count: conflict_entries.len(),
        conflicts: conflict_entries,
    };
    if out.json { out.print_json(&result); }
    else {
        if result.count == 0 {
            out.print_human("No conflicts");
        } else {
            out.print_human(&format!("{} conflict(s):", result.count));
            for c in &result.conflicts {
                out.print_human(&format!("  {}", c.path));
            }
        }
    }
    Ok(0)
}
