use anyhow::Result;
use chrono::DateTime;
use crate::cli::TagsArgs;
use crate::git_ctx::GitContext;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(_args: TagsArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let repo = &ctx.repo;
    let tag_names = repo.tag_names(None)?;
    let mut tags = Vec::new();

    for name in tag_names.iter().flatten() {
        let refname = format!("refs/tags/{}", name);
        if let Ok(reference) = repo.find_reference(&refname) {
            let target_oid = reference.target().unwrap_or(reference.peel(git2::ObjectType::Any)?.id());
            // Try to resolve as annotated tag
            let (tagger, date, message, is_annotated, commit_sha) =
                if let Ok(tag_obj) = repo.find_tag(target_oid) {
                    let tagger = tag_obj.tagger().and_then(|s| s.name().map(String::from));
                    let date = tag_obj.tagger().map(|s| {
                        DateTime::from_timestamp(s.when().seconds(), 0)
                            .map(|d| d.to_rfc3339()).unwrap_or_default()
                    });
                    let message = tag_obj.message().map(|m| m.trim().to_string());
                    let commit_sha = tag_obj.target_id().to_string();
                    (tagger, date, message, true, commit_sha)
                } else {
                    (None, None, None, false, target_oid.to_string())
                };

            tags.push(TagInfo { name: name.to_string(), sha: commit_sha, tagger, date, message, is_annotated });
        }
    }

    let result = TagsResult { count: tags.len(), tags };
    if out.json {
        out.print_json(&result);
    } else {
        for t in &result.tags {
            let suffix = if t.is_annotated { " (annotated)" } else { "" };
            out.print_human(&format!("{} {}{}", t.name, &t.sha[..7.min(t.sha.len())], suffix));
        }
    }
    Ok(0)
}
