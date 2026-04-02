use anyhow::Result;
use crate::cli::MergeArgs;
use crate::models::WriteResult;
use crate::output::OutputConfig;
use crate::formats::pdf;

pub fn execute(args: MergeArgs, out: &OutputConfig) -> Result<i32> {
    pdf::merge_pdfs(&args.files, &args.output)?;

    let result = WriteResult {
        file: args.output,
        format: "pdf".into(),
        message: format!("Merged {} PDF files", args.files.len()),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
