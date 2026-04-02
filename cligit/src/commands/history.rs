use anyhow::Result;
use chrono::DateTime;
use crate::cli::HistoryArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: HistoryArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    revwalk.push_head()?;

    let mut commits = Vec::new();
    for oid in revwalk {
        if commits.len() >= args.max { break; }
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        // Check if this commit modifies the file
        let (adds, dels) = if commit.parent_count() == 0 {
            if tree.get_path(std::path::Path::new(&args.file)).is_ok() {
                (1, 0) // File was added in initial commit
            } else { continue; }
        } else {
            let parent_tree = commit.parent(0)?.tree()?;
            let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;
            let mut found = false;
            let mut a = 0usize;
            let mut d = 0usize;
            for (di, delta) in diff.deltas().enumerate() {
                let dp = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("");
                let op = delta.old_file().path().and_then(|p| p.to_str()).unwrap_or("");
                if dp == args.file || op == args.file {
                    found = true;
                    if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, di) {
                        let (_, pa, pd) = patch.line_stats().unwrap_or((0, 0, 0));
                        a = pa; d = pd;
                    }
                    break;
                }
            }
            if !found { continue; }
            (a, d)
        };

        let date = DateTime::from_timestamp(commit.time().seconds(), 0)
            .map(|d| d.to_rfc3339()).unwrap_or_default();
        commits.push(HistoryCommit {
            sha: oid.to_string(),
            message: commit.message().unwrap_or("").lines().next().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            date,
            additions: adds,
            deletions: dels,
        });
    }

    let result = HistoryResult { file: args.file, count: commits.len(), commits };
    if out.json {
        out.print_json(&result);
    } else {
        for c in &result.commits {
            out.print_human(&format!("{} {} - {}", &c.sha[..7.min(c.sha.len())], c.message, c.author));
        }
    }
    Ok(0)
}
