mod cli;
mod commands;
mod error;
mod format;
mod formats;
mod models;
mod output;

use clap::Parser;
use std::process;

use crate::cli::Cli;
use crate::error::OfficeError;
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

    let format_override = cli.format.as_deref();

    let exit_code = match commands::dispatch(cli.command, &out, format_override) {
        Ok(code) => code,
        Err(e) => {
            if let Some(office_err) = e.downcast_ref::<OfficeError>() {
                out.print_error(office_err);
                office_err.exit_code()
            } else {
                out.print_anyhow_error(&e);
                1
            }
        }
    };

    process::exit(exit_code);
}
