use anyhow::Result;
use crate::cli::FetchArgs;
use crate::git_ctx::GitContext;
use crate::git_shell::shell_git;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: FetchArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let mut cmd = vec!["fetch"];
    if args.all { cmd.push("--all"); }
    else { cmd.push(&args.remote); }
    if args.prune { cmd.push("--prune"); }

    let cmd_refs: Vec<&str> = cmd.iter().map(|s| *s).collect();
    let output = shell_git(&cmd_refs, &ctx.workdir)?;
    let result = WriteResult { message: format!("Fetched. {}", output.trim()) };
    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
