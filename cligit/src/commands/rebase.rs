use anyhow::Result;
use crate::cli::RebaseArgs;
use crate::git_ctx::GitContext;
use crate::git_shell::shell_git;
use crate::models::*;
use crate::output::OutputConfig;

pub fn execute(args: RebaseArgs, ctx: &GitContext, out: &OutputConfig) -> Result<i32> {
    let result = if args.abort {
        let output = shell_git(&["rebase", "--abort"], &ctx.workdir)?;
        WriteResult { message: format!("Rebase aborted. {}", output.trim()) }
    } else if args.r#continue {
        let output = shell_git(&["rebase", "--continue"], &ctx.workdir)?;
        WriteResult { message: format!("Rebase continued. {}", output.trim()) }
    } else {
        let output = shell_git(&["rebase", &args.onto], &ctx.workdir)?;
        WriteResult { message: format!("Rebased onto {}. {}", args.onto, output.trim()) }
    };

    if out.json { out.print_json(&result); }
    else { out.print_human(&result.message); }
    Ok(0)
}
