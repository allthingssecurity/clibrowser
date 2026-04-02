use anyhow::Result;
use crate::cli::RotateArgs;
use crate::formats::PageRange;
use crate::formats::pdf;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: RotateArgs, out: &OutputConfig) -> Result<i32> {
    let page_range = args.pages.as_ref().map(|p| PageRange::parse(p)).transpose()?;
    pdf::rotate_pdf(&args.file, page_range.as_ref(), args.angle, args.output.as_deref())?;

    let output = args.output.as_deref().unwrap_or(&args.file);
    let result = WriteResult {
        file: output.to_string(),
        format: "pdf".into(),
        message: format!("Rotated pages by {}°", args.angle),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
