use anyhow::Result;
use crate::cli::DiffArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: DiffArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;

    let diff = if let (Some(ref from), Some(ref to)) = (&args.from, &args.to) {
        let from_obj = repo.revparse_single(from)?;
        let to_obj = repo.revparse_single(to)?;
        let from_tree = from_obj.peel_to_tree()?;
        let to_tree = to_obj.peel_to_tree()?;
        repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?
    } else if let Some(ref from) = args.from {
        let from_obj = repo.revparse_single(from)?;
        let from_tree = from_obj.peel_to_tree()?;
        if args.staged {
            let mut index = repo.index()?;
            let index_tree_oid = index.write_tree()?;
            let index_tree = repo.find_tree(index_tree_oid)?;
            repo.diff_tree_to_tree(Some(&from_tree), Some(&index_tree), None)?
        } else {
            repo.diff_tree_to_workdir_with_index(Some(&from_tree), None)?
        }
    } else if args.staged {
        let head_tree = repo.head()?.peel_to_tree()?;
        repo.diff_tree_to_index(Some(&head_tree), None, None)?
    } else {
        repo.diff_index_to_workdir(None, None)?
    };

    let (files, stats) = parse_diff(&diff, args.stat);

    let result = DiffResult { stats, files };
    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("{} file(s) changed, {} insertions(+), {} deletions(-)",
            result.stats.files_changed, result.stats.additions, result.stats.deletions));
        for f in &result.files {
            out.print_human(&format!("  {} +{} -{} ({})", f.path, f.additions, f.deletions, f.status));
        }
    }
    Ok(0)
}

pub fn parse_diff(diff: &git2::Diff, stat_only: bool) -> (Vec<DiffFile>, DiffStats) {
    let mut files: Vec<DiffFile> = Vec::new();
    let mut total_add = 0usize;
    let mut total_del = 0usize;

    for (di, delta) in diff.deltas().enumerate() {
        let path = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("").to_string();
        let old_path = delta.old_file().path().and_then(|p| p.to_str()).map(|s| s.to_string());
        let old_path = if old_path.as_deref() == Some(path.as_str()) { None } else { old_path };
        let status = match delta.status() {
            git2::Delta::Added => "added",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Modified => "modified",
            git2::Delta::Renamed => "renamed",
            git2::Delta::Copied => "copied",
            _ => "unknown",
        }.to_string();

        let mut file_add = 0usize;
        let mut file_del = 0usize;
        let mut hunks = Vec::new();

        if let Ok(patch) = git2::Patch::from_diff(diff, di) {
            if let Some(patch) = patch {
                let (_, adds, dels) = patch.line_stats().unwrap_or((0, 0, 0));
                file_add = adds;
                file_del = dels;

                if !stat_only {
                    for hi in 0..patch.num_hunks() {
                        if let Ok((hunk, _)) = patch.hunk(hi) {
                            let header = std::str::from_utf8(hunk.header()).unwrap_or("").trim().to_string();
                            let mut lines = Vec::new();
                            for li in 0..patch.num_lines_in_hunk(hi).unwrap_or(0) {
                                if let Ok(line) = patch.line_in_hunk(hi, li) {
                                    let op = match line.origin() {
                                        '+' => "+",
                                        '-' => "-",
                                        _ => " ",
                                    }.to_string();
                                    let content = std::str::from_utf8(line.content()).unwrap_or("").to_string();
                                    lines.push(DiffLine { op, content });
                                }
                            }
                            hunks.push(DiffHunk {
                                old_start: hunk.old_start(),
                                old_lines: hunk.old_lines(),
                                new_start: hunk.new_start(),
                                new_lines: hunk.new_lines(),
                                header,
                                lines,
                            });
                        }
                    }
                }
            }
        }

        total_add += file_add;
        total_del += file_del;
        files.push(DiffFile { path, old_path, status, additions: file_add, deletions: file_del, hunks });
    }

    let stats = DiffStats { files_changed: files.len(), additions: total_add, deletions: total_del };
    (files, stats)
}
