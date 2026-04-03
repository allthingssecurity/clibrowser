use anyhow::Result;
use crate::cli::TreeArgs;
use crate::detect;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: TreeArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let head = ctx.repo.head()?.peel_to_commit()?;
    let tree = head.tree()?;

    let glob_pattern = args.pattern.as_ref().map(|p| glob::Pattern::new(p)).transpose()?;

    let mut entries: Vec<TreeEntry> = Vec::new();

    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        let name = match entry.name() {
            Some(n) => n,
            None => return git2::TreeWalkResult::Ok,
        };
        let path = if dir.is_empty() { name.to_string() } else { format!("{}{}", dir, name) };
        let depth = path.matches('/').count();

        // Depth limit
        if let Some(max_depth) = args.depth {
            if depth > max_depth {
                return git2::TreeWalkResult::Ok;
            }
        }

        let is_blob = entry.kind() == Some(git2::ObjectType::Blob);
        let is_tree = entry.kind() == Some(git2::ObjectType::Tree);

        // Filter noise
        if !args.no_filter && detect::should_skip_path(&path) {
            if is_tree {
                return git2::TreeWalkResult::Skip;
            }
            return git2::TreeWalkResult::Ok;
        }

        // Glob filter (only on files)
        if let Some(ref pat) = glob_pattern {
            if is_blob && !pat.matches(name) {
                return git2::TreeWalkResult::Ok;
            }
        }

        let size = if is_blob && args.sizes {
            ctx.repo.find_blob(entry.id()).ok().map(|b| b.content().len())
        } else {
            None
        };

        let entry_type = if is_blob { "file" } else if is_tree { "dir" } else { return git2::TreeWalkResult::Ok; };

        entries.push(TreeEntry {
            path: path.clone(),
            entry_type: entry_type.to_string(),
            size,
            depth,
        });

        git2::TreeWalkResult::Ok
    })?;

    let result = TreeResult {
        count: entries.len(),
        entries,
    };

    if out.json {
        out.print_json(&result);
    } else {
        for entry in &result.entries {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.entry_type == "dir" { "/" } else { "" };
            let basename = entry.path.rsplit('/').next().unwrap_or(&entry.path);
            let size_str = entry.size.map(|s| format!(" ({})", format_size(s))).unwrap_or_default();
            out.print_human(&format!("{}{}{}{}", indent, basename, icon, size_str));
        }
        out.print_human(&format!("\n{} entries", result.count));
    }
    Ok(0)
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 { format!("{}B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1}K", bytes as f64 / 1024.0) }
    else { format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0)) }
}
