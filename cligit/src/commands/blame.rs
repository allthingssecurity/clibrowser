use anyhow::Result;
use chrono::DateTime;
use crate::cli::BlameArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: BlameArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let mut opts = git2::BlameOptions::new();
    if let Some(ref r) = args.r#ref {
        let obj = repo.revparse_single(r)?;
        opts.newest_commit(obj.id());
    }

    let blame = repo.blame_file(std::path::Path::new(&args.file), Some(&mut opts))?;

    // Read the file content to get line text
    let file_path = ctx.workdir.join(&args.file);
    let content = std::fs::read_to_string(&file_path).unwrap_or_default();
    let content_lines: Vec<&str> = content.lines().collect();

    let mut lines = Vec::new();
    for (i, hunk) in blame.iter().enumerate() {
        let sig = hunk.final_signature();
        let sha = hunk.final_commit_id().to_string();
        let date = DateTime::from_timestamp(sig.when().seconds(), 0)
            .map(|d| d.to_rfc3339()).unwrap_or_default();
        let line_content = content_lines.get(i).unwrap_or(&"").to_string();
        lines.push(BlameLineInfo {
            line_no: i + 1,
            sha: sha[..7.min(sha.len())].to_string(),
            author: sig.name().unwrap_or("").to_string(),
            date,
            content: line_content,
        });
    }

    let result = BlameResult { file: args.file.clone(), lines };
    if out.json {
        out.print_json(&result);
    } else {
        for l in &result.lines {
            out.print_human(&format!("{} ({} {}) {}", l.sha, l.author, &l.date[..10.min(l.date.len())], l.content));
        }
    }
    Ok(0)
}
