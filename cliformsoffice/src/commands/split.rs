use anyhow::Result;
use crate::cli::SplitArgs;
use crate::formats::PageRange;
use crate::formats::pdf;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: SplitArgs, out: &OutputConfig) -> Result<i32> {
    let range = PageRange::parse(&args.pages)?;
    pdf::split_pdf(&args.file, &range, &args.output)?;

    let result = WriteResult {
        file: args.output,
        format: "pdf".into(),
        message: format!("Split pages {} from {}", args.pages, args.file),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
