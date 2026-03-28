mod cli;
mod commands;
mod dom;
mod error;
mod http;
mod output;
mod session;

use clap::Parser;
use std::process;

use crate::cli::Cli;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut cli = Cli::parse();
    let stealth = cli.stealth;

    let out = OutputConfig {
        json: cli.json,
        quiet: cli.quiet,
    };

    // Propagate global --stealth to commands that use GetArgs
    propagate_stealth(&mut cli.command, stealth);

    let mut session = match Session::load(&cli.session) {
        Ok(s) => s,
        Err(e) => {
            out.print_anyhow_error(&e);
            process::exit(1);
        }
    };

    let exit_code = match commands::dispatch(cli.command, &mut session, &out, stealth).await {
        Ok(code) => {
            if let Err(e) = session.save() {
                out.print_anyhow_error(&e);
                1
            } else {
                code
            }
        }
        Err(e) => {
            let _ = session.save();

            if let Some(browser_err) = e.downcast_ref::<BrowserError>() {
                out.print_error(browser_err);
                browser_err.exit_code()
            } else {
                out.print_anyhow_error(&e);
                1
            }
        }
    };

    process::exit(exit_code);
}

fn propagate_stealth(cmd: &mut cli::Command, stealth: bool) {
    match cmd {
        cli::Command::Get(ref mut args) => args.stealth = stealth,
        _ => {}
    }
}
