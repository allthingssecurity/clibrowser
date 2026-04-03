use anyhow::Result;
use std::collections::HashMap;
use chrono::DateTime;
use crate::cli::ContextArgs;
use crate::detect;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: ContextArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo_name = ctx.workdir.file_name()
        .and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let head = ctx.repo.head()?.peel_to_commit()?;
    let tree = head.tree()?;

    let mut lang_stats: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut tree_lines: Vec<String> = Vec::new();
    let mut key_files: Vec<String> = Vec::new();
    let mut readme_content: Option<String> = None;
    let mut deps_summary: Vec<String> = Vec::new();

    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        let name = match entry.name() {
            Some(n) => n,
            None => return git2::TreeWalkResult::Ok,
        };
        let path = if dir.is_empty() { name.to_string() } else { format!("{}{}", dir, name) };

        if detect::should_skip_path(&path) {
            if entry.kind() == Some(git2::ObjectType::Tree) {
                return git2::TreeWalkResult::Skip;
            }
            return git2::TreeWalkResult::Ok;
        }

        if entry.kind() == Some(git2::ObjectType::Blob) {
            // Tree display
            if args.include_tree && tree_lines.len() < 100 {
                tree_lines.push(path.clone());
            }

            // Language stats
            if let Some(ext) = std::path::Path::new(name).extension().and_then(|e| e.to_str()) {
                if let Some(lang) = detect::detect_language(ext) {
                    let lines = ctx.repo.find_blob(entry.id())
                        .map(|b| detect::count_lines(b.content()))
                        .unwrap_or(0);
                    let e = lang_stats.entry(lang).or_insert((0, 0));
                    e.0 += 1;
                    e.1 += lines;
                }
            }

            // README
            if args.include_readme && readme_content.is_none()
                && (name.eq_ignore_ascii_case("readme.md") || name.eq_ignore_ascii_case("readme"))
                && dir.is_empty()
            {
                if let Ok(blob) = ctx.repo.find_blob(entry.id()) {
                    let content = String::from_utf8_lossy(blob.content());
                    let truncated: String = content.chars().take(2000).collect();
                    readme_content = Some(truncated);
                }
            }

            // Key files
            if detect::ENTRY_POINTS.contains(&name) || detect::CONFIG_FILES.contains(&name) {
                key_files.push(path.clone());
            }

            // Deps
            if args.include_deps {
                if let Some(manager) = detect::detect_manifest(name) {
                    if let Ok(blob) = ctx.repo.find_blob(entry.id()) {
                        let content = String::from_utf8_lossy(blob.content());
                        let parsed = super::deps::parse_manifest_deps(manager, &content);
                        for dep in &parsed {
                            deps_summary.push(dep.name.clone());
                        }
                    }
                }
            }
        }

        git2::TreeWalkResult::Ok
    })?;

    // Build language info (top 5)
    let total_lines: usize = lang_stats.values().map(|v| v.1).sum();
    let mut languages: Vec<LanguageInfo> = lang_stats.into_iter().map(|(lang, (files, lines))| {
        let percentage = if total_lines > 0 { (lines as f64 / total_lines as f64) * 100.0 } else { 0.0 };
        LanguageInfo { language: lang.to_string(), files, lines, percentage }
    }).collect();
    languages.sort_by(|a, b| b.lines.cmp(&a.lines));
    languages.truncate(5);

    // Recent commits
    let mut recent_commits: Vec<String> = Vec::new();
    if let Ok(mut revwalk) = ctx.repo.revwalk() {
        let _ = revwalk.set_sorting(git2::Sort::TIME);
        let _ = revwalk.push_head();
        for oid in revwalk.take(5) {
            if let Ok(oid) = oid {
                if let Ok(commit) = ctx.repo.find_commit(oid) {
                    let msg = commit.message().unwrap_or("").lines().next().unwrap_or("").to_string();
                    let date = DateTime::from_timestamp(commit.time().seconds(), 0)
                        .map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
                    recent_commits.push(format!("{} {}", date, msg));
                }
            }
        }
    }

    // Description from README first paragraph
    let description = readme_content.as_ref().and_then(|r| {
        r.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .next()
            .map(|l| l.trim().to_string())
    });

    let file_tree = tree_lines.join("\n");

    // Build text output
    let mut text = String::new();
    text.push_str(&format!("# {}\n\n", repo_name));
    if let Some(desc) = &description {
        text.push_str(&format!("{}\n\n", desc));
    }

    text.push_str("## Languages\n");
    for lang in &languages {
        text.push_str(&format!("- {} ({:.1}%, {} files, {} lines)\n", lang.language, lang.percentage, lang.files, lang.lines));
    }

    if !key_files.is_empty() {
        text.push_str("\n## Key files\n");
        for f in &key_files {
            text.push_str(&format!("- {}\n", f));
        }
    }

    if !deps_summary.is_empty() {
        text.push_str("\n## Dependencies\n");
        text.push_str(&deps_summary.join(", "));
        text.push('\n');
    }

    if !file_tree.is_empty() {
        text.push_str("\n## File tree\n```\n");
        text.push_str(&file_tree);
        text.push_str("\n```\n");
    }

    if let Some(readme) = &readme_content {
        text.push_str("\n## README\n");
        text.push_str(readme);
        text.push('\n');
    }

    if !recent_commits.is_empty() {
        text.push_str("\n## Recent commits\n");
        for c in &recent_commits {
            text.push_str(&format!("- {}\n", c));
        }
    }

    // Truncate to max_length
    if text.len() > args.max_length {
        text.truncate(args.max_length);
        text.push_str("\n...(truncated)");
    }

    let total_length = text.len();

    if out.json {
        let result = ContextResult {
            repo_name,
            description,
            languages,
            file_tree,
            key_files,
            deps_summary,
            readme: readme_content,
            recent_commits,
            total_length,
        };
        out.print_json(&result);
    } else {
        out.print_human(&text);
    }
    Ok(0)
}
