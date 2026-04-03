use anyhow::Result;
use std::collections::HashMap;
use crate::cli::LanguagesArgs;
use crate::detect;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: LanguagesArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let head = ctx.repo.head()?.peel_to_commit()?;
    let tree = head.tree()?;

    // language -> (file_count, line_count)
    let mut stats: HashMap<&str, (usize, usize)> = HashMap::new();

    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() != Some(git2::ObjectType::Blob) {
            return git2::TreeWalkResult::Ok;
        }
        let name = match entry.name() {
            Some(n) => n,
            None => return git2::TreeWalkResult::Ok,
        };
        let path = if dir.is_empty() { name.to_string() } else { format!("{}{}", dir, name) };

        if detect::should_skip_path(&path) {
            return git2::TreeWalkResult::Ok;
        }

        let ext = match std::path::Path::new(name).extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => return git2::TreeWalkResult::Ok,
        };

        if let Some(lang) = detect::detect_language(ext) {
            let lines = ctx.repo.find_blob(entry.id())
                .map(|b| detect::count_lines(b.content()))
                .unwrap_or(0);
            let entry = stats.entry(lang).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += lines;
        }
        git2::TreeWalkResult::Ok
    })?;

    let total_files: usize = stats.values().map(|v| v.0).sum();
    let total_lines: usize = stats.values().map(|v| v.1).sum();

    let mut languages: Vec<LanguageInfo> = stats.into_iter().map(|(lang, (files, lines))| {
        let percentage = if total_lines > 0 { (lines as f64 / total_lines as f64) * 100.0 } else { 0.0 };
        LanguageInfo { language: lang.to_string(), files, lines, percentage }
    }).collect();
    languages.sort_by(|a, b| b.lines.cmp(&a.lines));

    let result = LanguagesResult {
        count: languages.len(),
        total_files,
        total_lines,
        languages,
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("{} languages, {} files, {} lines\n", result.count, total_files, total_lines));
        for lang in &result.languages {
            out.print_human(&format!("  {:>5.1}%  {:12}  {:>6} files  {:>8} lines", lang.percentage, lang.language, lang.files, lang.lines));
        }
    }
    Ok(0)
}
