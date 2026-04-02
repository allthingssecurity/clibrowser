use anyhow::Result;
use crate::cli::WatermarkArgs;
use crate::error::OfficeError;
use crate::output::OutputConfig;

pub fn execute(_args: WatermarkArgs, out: &OutputConfig, _format_override: Option<&str>) -> Result<i32> {
    let msg = "Watermark support is planned for a future release. \
               This operation requires graphical rendering capabilities \
               that are not yet implemented.";

    if out.json {
        let obj = serde_json::json!({
            "ok": false,
            "error": "not_supported",
            "message": msg,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        out.print_human(msg);
    }

    Err(OfficeError::NotSupported {
        op: "watermark".into(),
        format: "all formats".into(),
    }.into())
}
