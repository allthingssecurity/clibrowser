mod cli;
mod commands;
mod error;
mod git_ctx;
mod git_shell;
mod models;
mod output;

use clap::Parser;
use std::process;

use crate::cli::{Cli, Command};
use crate::error::GitError;
use crate::git_ctx::GitContext;
use crate::output::OutputConfig;

fn main() {
    // Handle SIGPIPE gracefully
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse();

    let out = OutputConfig {
        json: cli.json,
        quiet: cli.quiet,
    };

    // Clone command doesn't need a repo context
    if let Command::Clone(args) = cli.command {
        let exit_code = match commands::clone_cmd::execute(args, &out) {
            Ok(code) => code,
            Err(e) => handle_error(&out, e),
        };
        process::exit(exit_code);
    }

    // All other commands need a repo
    let mut ctx = match GitContext::open(cli.directory.as_deref()) {
        Ok(ctx) => ctx,
        Err(e) => {
            out.print_error(&e);
            process::exit(e.exit_code());
        }
    };

    let exit_code = match commands::dispatch(cli.command, &mut ctx, &out) {
        Ok(code) => code,
        Err(e) => handle_error(&out, e),
    };

    process::exit(exit_code);
}

fn handle_error(out: &OutputConfig, e: anyhow::Error) -> i32 {
    if let Some(git_err) = e.downcast_ref::<GitError>() {
        out.print_error(git_err);
        git_err.exit_code()
    } else {
        out.print_anyhow_error(&e);
        1
    }
}
