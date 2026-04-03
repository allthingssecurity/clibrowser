use anyhow::Result;
use crate::cli::DepsArgs;
use crate::detect;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: DepsArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let head = ctx.repo.head()?.peel_to_commit()?;
    let tree = head.tree()?;

    let mut manifests: Vec<ManifestInfo> = Vec::new();

    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() != Some(git2::ObjectType::Blob) {
            return git2::TreeWalkResult::Ok;
        }
        let name = match entry.name() {
            Some(n) => n,
            None => return git2::TreeWalkResult::Ok,
        };

        if let Some(manager) = detect::detect_manifest(name) {
            let path = if dir.is_empty() { name.to_string() } else { format!("{}{}", dir, name) };
            if let Ok(blob) = ctx.repo.find_blob(entry.id()) {
                let content = String::from_utf8_lossy(blob.content());
                let deps = parse_deps(manager, &content);
                manifests.push(ManifestInfo {
                    file: path,
                    manager: manager.to_string(),
                    deps,
                });
            }
        }
        git2::TreeWalkResult::Ok
    })?;

    let total_deps: usize = manifests.iter().map(|m| m.deps.len()).sum();

    let result = DepsResult {
        manifest_count: manifests.len(),
        total_deps,
        manifests,
    };

    if out.json {
        out.print_json(&result);
    } else {
        out.print_human(&format!("{} manifests, {} dependencies\n", result.manifest_count, result.total_deps));
        for manifest in &result.manifests {
            out.print_human(&format!("  {} ({})", manifest.file, manifest.manager));
            for dep in &manifest.deps {
                let ver = dep.version.as_deref().unwrap_or("*");
                out.print_human(&format!("    {} {} [{}]", dep.name, ver, dep.dep_type));
            }
        }
    }
    Ok(0)
}

pub fn parse_manifest_deps(manager: &str, content: &str) -> Vec<DependencyInfo> {
    parse_deps(manager, content)
}

fn parse_deps(manager: &str, content: &str) -> Vec<DependencyInfo> {
    match manager {
        "npm" => parse_package_json(content),
        "cargo" => parse_cargo_toml(content),
        "pip" => parse_requirements_txt(content),
        "go" => parse_go_mod(content),
        "ruby" => parse_gemfile(content),
        _ => Vec::new(),
    }
}

fn parse_package_json(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    let val: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return deps,
    };
    if let Some(obj) = val.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in obj {
            deps.push(DependencyInfo {
                name: name.clone(),
                version: version.as_str().map(String::from),
                dep_type: "runtime".to_string(),
            });
        }
    }
    if let Some(obj) = val.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, version) in obj {
            deps.push(DependencyInfo {
                name: name.clone(),
                version: version.as_str().map(String::from),
                dep_type: "dev".to_string(),
            });
        }
    }
    deps
}

fn parse_cargo_toml(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    let mut in_section: Option<&str> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = match trimmed {
                "[dependencies]" => Some("runtime"),
                "[dev-dependencies]" => Some("dev"),
                "[build-dependencies]" => Some("build"),
                _ => None,
            };
            continue;
        }
        if let Some(dep_type) = in_section {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[..eq_pos].trim().to_string();
                let val_part = trimmed[eq_pos + 1..].trim();
                let version = if val_part.starts_with('"') {
                    val_part.trim_matches('"').to_string()
                } else if let Some(start) = val_part.find("version") {
                    let after = &val_part[start..];
                    after.split('"').nth(1).unwrap_or("*").to_string()
                } else {
                    "*".to_string()
                };
                deps.push(DependencyInfo {
                    name,
                    version: Some(version),
                    dep_type: dep_type.to_string(),
                });
            }
        }
    }
    deps
}

fn parse_requirements_txt(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }
        let (name, version) = if let Some(pos) = trimmed.find("==") {
            (trimmed[..pos].trim().to_string(), Some(trimmed[pos + 2..].trim().to_string()))
        } else if let Some(pos) = trimmed.find(">=") {
            (trimmed[..pos].trim().to_string(), Some(format!(">={}", trimmed[pos + 2..].trim())))
        } else if let Some(pos) = trimmed.find("~=") {
            (trimmed[..pos].trim().to_string(), Some(format!("~={}", trimmed[pos + 2..].trim())))
        } else {
            (trimmed.to_string(), None)
        };
        deps.push(DependencyInfo { name, version, dep_type: "runtime".to_string() });
    }
    deps
}

fn parse_go_mod(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    let mut in_require = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("require (") || trimmed == "require (" {
            in_require = true;
            continue;
        }
        if trimmed == ")" {
            in_require = false;
            continue;
        }
        if in_require {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(DependencyInfo {
                    name: parts[0].to_string(),
                    version: Some(parts[1].to_string()),
                    dep_type: "runtime".to_string(),
                });
            }
        }
    }
    deps
}

fn parse_gemfile(content: &str) -> Vec<DependencyInfo> {
    let mut deps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("gem ") || trimmed.starts_with("gem\t") {
            let rest = trimmed[3..].trim();
            // gem "name" or gem 'name', "version"
            let quote = if rest.starts_with('"') { '"' } else if rest.starts_with('\'') { '\'' } else { continue };
            let after_first = &rest[1..];
            if let Some(end) = after_first.find(quote) {
                let name = after_first[..end].to_string();
                let remaining = after_first[end + 1..].trim();
                let version = if remaining.starts_with(',') {
                    let ver_part = remaining[1..].trim();
                    let vq = if ver_part.starts_with('"') { '"' } else if ver_part.starts_with('\'') { '\'' } else { ' ' };
                    if vq != ' ' {
                        ver_part[1..].split(vq).next().map(String::from)
                    } else {
                        None
                    }
                } else {
                    None
                };
                deps.push(DependencyInfo { name, version, dep_type: "runtime".to_string() });
            }
        }
    }
    deps
}
