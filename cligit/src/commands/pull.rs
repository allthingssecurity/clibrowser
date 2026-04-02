use anyhow::Result;
use crate::cli::PullArgs;
use crate::git_ctx::GitContext;
use crate::git_shell::shell_git;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: PullArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let mut cmd_parts: Vec<String> = vec!["pull".into()];
    if args.rebase { cmd_parts.push("--rebase".into()); }
    if let Some(ref r) = args.remote { cmd_parts.push(r.clone()); }
    if let Some(ref b) = args.branch { cmd_parts.push(b.clone()); }

    let cmd_refs: Vec<&str> = cmd_parts.iter().map(|s| s.as_str()).collect();
    let output = shell_git(&cmd_refs, &ctx.workdir)?;
    let result = WriteResult { message: format!("Pulled. {}", output.trim()) };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
