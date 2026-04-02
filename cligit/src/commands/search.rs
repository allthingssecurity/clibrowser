use anyhow::Result;
use regex::Regex;
use crate::cli::SearchArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: SearchArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let re = Regex::new(&args.pattern)?;

    if args.mode == "grep" {
        let obj = repo.revparse_single(&args.r#ref)?;
        let tree = obj.peel_to_tree()?;
        let mut matches = Vec::new();

        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if matches.len() >= args.max { return git2::TreeWalkResult::Abort; }
            if entry.kind() != Some(git2::ObjectType::Blob) { return git2::TreeWalkResult::Ok; }
            let path = format!("{}{}", dir, entry.name().unwrap_or(""));
            if let Ok(blob) = repo.find_blob(entry.id()) {
                if let Ok(content) = std::str::from_utf8(blob.content()) {
                    for (i, line) in content.lines().enumerate() {
                        if matches.len() >= args.max { break; }
                        if re.is_match(line) {
                            matches.push(SearchMatch {
                                file: path.clone(),
                                line_no: i + 1,
                                content: line.to_string(),
                                context_before: None,
                                context_after: None,
                            });
                        }
                    }
                }
            }
            git2::TreeWalkResult::Ok
        })?;

        let count = matches.len();
        let result = SearchResult {
            pattern: args.pattern, mode: "grep".into(), count,
            matches: serde_json::to_value(&matches)?,
        };
        if out.json { out.print_json(&result); }
        else { for m in &matches { out.print_human(&format!("{}:{}: {}", m.file, m.line_no, m.content)); } }
    } else {
        // pickaxe mode - find commits that add/remove the pattern
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;
        let mut commit_matches = Vec::new();
        for oid in revwalk {
            if commit_matches.len() >= args.max { break; }
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            let tree = commit.tree()?;
            let parent_tree = if commit.parent_count() > 0 { Some(commit.parent(0)?.tree()?) } else { None };
            let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
            let mut found = false;
            diff.foreach(&mut |_, _| true, None, None, Some(&mut |_delta, _hunk, line| {
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    if re.is_match(content) { found = true; }
                }
                true
            }))?;
            if found {
                let sha = oid.to_string();
                commit_matches.push(CommitInfo {
                    short_sha: sha[..7.min(sha.len())].to_string(), sha,
                    message: commit.message().unwrap_or("").lines().next().unwrap_or("").to_string(),
                    body: None,
                    author: AuthorInfo { name: commit.author().name().unwrap_or("").into(), email: commit.author().email().unwrap_or("").into() },
                    date: chrono::DateTime::from_timestamp(commit.time().seconds(), 0).map(|d| d.to_rfc3339()).unwrap_or_default(),
                    parents: commit.parent_ids().map(|p| p.to_string()).collect(),
                    is_merge: commit.parent_count() > 1,
                });
            }
        }
        let count = commit_matches.len();
        let result = SearchResult {
            pattern: args.pattern, mode: "pickaxe".into(), count,
            matches: serde_json::to_value(&commit_matches)?,
        };
        if out.json { out.print_json(&result); }
        else { for c in &commit_matches { out.print_human(&format!("{} {}", c.short_sha, c.message)); } }
    }
    Ok(0)
}
