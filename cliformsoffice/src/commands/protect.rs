use anyhow::Result;
use crate::cli::ProtectArgs;
use crate::formats::pdf;
use crate::output::OutputConfig;

pub fn execute(args: ProtectArgs, out: &OutputConfig) -> Result<i32> {
    pdf::protect_pdf(&args.file, &args.password, args.output.as_deref())?;
    // protect_pdf currently returns an error (not supported)
    // This line is reached only if it succeeds in the future
    let _ = out;
    Ok(0)
}
