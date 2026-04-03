use anyhow::Result;
use std::collections::HashMap;
use crate::cli::AnalyzeArgs;
use crate::detect;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: AnalyzeArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo_name = ctx.workdir.file_name()
        .and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    let head = ctx.repo.head()?.peel_to_commit()?;
    let tree = head.tree()?;

    let mut lang_stats: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut file_count = 0usize;
    let mut key_files: Vec<KeyFile> = Vec::new();
    let mut file_tree: Vec<String> = Vec::new();
    let mut readme_content: Option<String> = None;
    let mut manifest_files: Vec<(String, &str, String)> = Vec::new(); // (path, manager, content)

    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        let name = match entry.name() {
            Some(n) => n,
            None => return git2::TreeWalkResult::Ok,
        };
        let path = if dir.is_empty() { name.to_string() } else { format!("{}{}", dir, name) };

        // Collect tree entries (first 200)
        if file_tree.len() < 200 {
            let is_blob = entry.kind() == Some(git2::ObjectType::Blob);
            if is_blob || entry.kind() == Some(git2::ObjectType::Tree) {
                if !detect::should_skip_path(&path) {
                    file_tree.push(path.clone());
                }
            }
        }

        if entry.kind() != Some(git2::ObjectType::Blob) {
            return git2::TreeWalkResult::Ok;
        }

        if detect::should_skip_path(&path) {
            return git2::TreeWalkResult::Ok;
        }

        file_count += 1;

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
        if readme_content.is_none() && (name.eq_ignore_ascii_case("readme.md") || name.eq_ignore_ascii_case("readme")) && dir.is_empty() {
            if let Ok(blob) = ctx.repo.find_blob(entry.id()) {
                let content = String::from_utf8_lossy(blob.content());
                let truncated: String = content.chars().take(2000).collect();
                readme_content = Some(truncated);
            }
        }

        // Key files
        if detect::ENTRY_POINTS.contains(&name) {
            let lines = ctx.repo.find_blob(entry.id()).ok().map(|b| detect::count_lines(b.content()));
            key_files.push(KeyFile { path: path.clone(), category: "entry_point".to_string(), lines });
        } else if detect::CONFIG_FILES.contains(&name) {
            key_files.push(KeyFile { path: path.clone(), category: "config".to_string(), lines: None });
        } else if detect::DOC_FILES.contains(&name) {
            key_files.push(KeyFile { path: path.clone(), category: "docs".to_string(), lines: None });
        }

        // Manifests
        if let Some(manager) = detect::detect_manifest(name) {
            if let Ok(blob) = ctx.repo.find_blob(entry.id()) {
                let content = String::from_utf8_lossy(blob.content()).to_string();
                manifest_files.push((path.clone(), manager, content));
            }
        }

        git2::TreeWalkResult::Ok
    })?;

    // Build language info
    let total_lines: usize = lang_stats.values().map(|v| v.1).sum();
    let mut languages: Vec<LanguageInfo> = lang_stats.into_iter().map(|(lang, (files, lines))| {
        let percentage = if total_lines > 0 { (lines as f64 / total_lines as f64) * 100.0 } else { 0.0 };
        LanguageInfo { language: lang.to_string(), files, lines, percentage }
    }).collect();
    languages.sort_by(|a, b| b.lines.cmp(&a.lines));

    // Tech stack
    let mut tech_stack: Vec<String> = Vec::new();
    for lang in &languages {
        if !tech_stack.contains(&lang.language) {
            tech_stack.push(lang.language.clone());
        }
    }
    for (_, manager, _) in &manifest_files {
        let mgr = manager.to_string();
        if !tech_stack.contains(&mgr) {
            tech_stack.push(mgr);
        }
    }

    // Parse deps
    let deps: Vec<ManifestInfo> = manifest_files.into_iter().map(|(path, manager, content)| {
        let parsed = super::deps::parse_manifest_deps(manager, &content);
        ManifestInfo { file: path, manager: manager.to_string(), deps: parsed }
    }).collect();

    // Description from README first paragraph
    let description = readme_content.as_ref().and_then(|r| {
        r.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .next()
            .map(|l| l.trim().to_string())
    });

    let result = AnalyzeResult {
        name: repo_name,
        description,
        languages,
        tech_stack,
        key_files,
        file_count,
        total_lines,
        deps,
        readme_excerpt: readme_content,
        file_tree,
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("Repository: {}", result.name));
        if let Some(desc) = &result.description {
            out.print_human(&format!("  {}", desc));
        }
        out.print_human(&format!("\n{} files, {} lines", result.file_count, result.total_lines));
        out.print_human("\nLanguages:");
        for lang in &result.languages {
            out.print_human(&format!("  {:>5.1}%  {}", lang.percentage, lang.language));
        }
        out.print_human(&format!("\nTech stack: {}", result.tech_stack.join(", ")));
        if !result.key_files.is_empty() {
            out.print_human("\nKey files:");
            for kf in &result.key_files {
                out.print_human(&format!("  [{}] {}", kf.category, kf.path));
            }
        }
        if !result.deps.is_empty() {
            out.print_human(&format!("\n{} dependency manifests", result.deps.len()));
        }
    }
    Ok(0)
}
