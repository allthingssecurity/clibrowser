use anyhow::Result;
use chrono::DateTime;
use std::collections::HashMap;
use crate::cli::ContributorsArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

struct ContribAccum {
    name: String,
    email: String,
    commits: usize,
    additions: usize,
    deletions: usize,
    first: i64,
    last: i64,
}

pub fn execute(args: ContributorsArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    revwalk.push_head()?;

    let mut map: HashMap<String, ContribAccum> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let email = commit.author().email().unwrap_or("").to_string();
        let name = commit.author().name().unwrap_or("").to_string();
        let ts = commit.time().seconds();

        let entry = map.entry(email.clone()).or_insert(ContribAccum {
            name: name.clone(), email, commits: 0, additions: 0, deletions: 0, first: ts, last: ts,
        });
        entry.commits += 1;
        if ts < entry.first { entry.first = ts; }
        if ts > entry.last { entry.last = ts; }
    }

    let mut contributors: Vec<ContributorInfo> = map.into_values().map(|c| {
        ContributorInfo {
            name: c.name,
            email: c.email,
            commits: c.commits,
            additions: c.additions,
            deletions: c.deletions,
            first_commit: DateTime::from_timestamp(c.first, 0).map(|d| d.to_rfc3339()).unwrap_or_default(),
            last_commit: DateTime::from_timestamp(c.last, 0).map(|d| d.to_rfc3339()).unwrap_or_default(),
        }
    }).collect();
    contributors.sort_by(|a, b| b.commits.cmp(&a.commits));
    contributors.truncate(args.max);

    let result = ContributorsResult { count: contributors.len(), contributors };
    if out.json {
        out.print_json(&result);
    } else {
        for c in &result.contributors {
            out.print_human(&format!("{} <{}> {} commits", c.name, c.email, c.commits));
        }
    }
    Ok(0)
}
