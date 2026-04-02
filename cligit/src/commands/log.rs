use anyhow::Result;
use chrono::DateTime;
use crate::cli::LogArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

fn time_to_string(t: git2::Time) -> String {
    DateTime::from_timestamp(t.seconds(), 0)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}

pub fn execute(args: LogArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    if let Some(ref r) = args.r#ref {
        let obj = repo.revparse_single(r)?;
        revwalk.push(obj.id())?;
    } else {
        revwalk.push_head()?;
    }

    let since_ts = args.since.as_ref().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp());
    let until_ts = args.until.as_ref().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp());
    let grep_re = args.grep.as_ref().and_then(|p| regex::Regex::new(p).ok());
    let author_re = args.author.as_ref().and_then(|p| regex::Regex::new(p).ok());

    let mut commits = Vec::new();
    for oid in revwalk {
        if commits.len() >= args.max { break; }
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let ts = commit.time().seconds();

        if let Some(s) = since_ts { if ts < s { continue; } }
        if let Some(u) = until_ts { if ts > u { continue; } }

        let msg = commit.message().unwrap_or("").to_string();
        let author_name = commit.author().name().unwrap_or("").to_string();

        if let Some(ref re) = grep_re { if !re.is_match(&msg) { continue; } }
        if let Some(ref re) = author_re { if !re.is_match(&author_name) { continue; } }

        if let Some(ref path) = args.path {
            let dominated = commit_touches_path(repo, &commit, path);
            if !dominated { continue; }
        }

        let sha = oid.to_string();
        let parents: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();
        let (first_line, body) = split_message(&msg);
        commits.push(CommitInfo {
            short_sha: sha[..7.min(sha.len())].to_string(),
            sha,
            message: first_line,
            body,
            author: AuthorInfo {
                name: author_name,
                email: commit.author().email().unwrap_or("").to_string(),
            },
            date: time_to_string(commit.time()),
            is_merge: parents.len() > 1,
            parents,
        });
    }

    let result = LogResult { count: commits.len(), commits };
    if out.json {
        out.print_json(&result);
    } else {
        for c in &result.commits {
            out.print_human(&format!("{} {} - {} ({})", c.short_sha, c.message, c.author.name, &c.date[..10.min(c.date.len())]));
        }
    }
    Ok(0)
}

fn split_message(msg: &str) -> (String, Option<String>) {
    let trimmed = msg.trim();
    if let Some(idx) = trimmed.find('\n') {
        let first = trimmed[..idx].trim().to_string();
        let rest = trimmed[idx+1..].trim().to_string();
        (first, if rest.is_empty() { None } else { Some(rest) })
    } else {
        (trimmed.to_string(), None)
    }
}

fn commit_touches_path(repo: &git2::Repository, commit: &git2::Commit, path: &str) -> bool {
    let tree = match commit.tree() { Ok(t) => t, Err(_) => return false };
    if commit.parent_count() == 0 {
        return tree.get_path(std::path::Path::new(path)).is_ok();
    }
    for i in 0..commit.parent_count() {
        if let Ok(parent) = commit.parent(i) {
            if let Ok(parent_tree) = parent.tree() {
                if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) {
                    for delta in diff.deltas() {
                        let dp = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("");
                        if dp == path { return true; }
                    }
                }
            }
        }
    }
    false
}
