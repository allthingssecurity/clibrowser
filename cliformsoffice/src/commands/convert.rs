use std::path::Path;
use anyhow::Result;
use crate::cli::ConvertArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::get_backend;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: ConvertArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file.clone()).into());
    }

    let kind = detect_format(path, format_override)?;

    let output_path = match &args.output {
        Some(o) => o.clone(),
        None => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            format!("{}.{}", stem, args.to)
        }
    };

    match args.to.as_str() {
        "md" | "markdown" => convert_to_markdown(path, &output_path, kind, out),
        "csv" => convert_to_csv(path, &output_path, kind, out),
        "txt" | "text" => convert_to_text(path, &output_path, kind, out),
        _ => {
            // Try LibreOffice for other conversions
            convert_via_libreoffice(path, &output_path, &args.to, out)
        }
    }
}

fn convert_to_markdown(path: &Path, output: &str, kind: crate::format::FormatKind, out: &OutputConfig) -> Result<i32> {
    let backend = get_backend(kind)?;
    let md = backend.markdown(path, None)?;
    std::fs::write(output, &md)?;

    let result = WriteResult {
        file: output.to_string(),
        format: "md".into(),
        message: format!("Converted {} to markdown", path.display()),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn convert_to_csv(path: &Path, output: &str, kind: crate::format::FormatKind, out: &OutputConfig) -> Result<i32> {
    let backend = get_backend(kind)?;
    let tables = backend.tables(path, None)?;

    if tables.is_empty() {
        return Err(OfficeError::Other("No tables found to convert".into()).into());
    }

    let mut wtr = csv::Writer::from_path(output)?;
    for row in &tables[0].data {
        wtr.write_record(row)?;
    }
    wtr.flush()?;

    let result = WriteResult {
        file: output.to_string(),
        format: "csv".into(),
        message: format!("Converted first table to CSV ({} rows)", tables[0].rows),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn convert_to_text(path: &Path, output: &str, kind: crate::format::FormatKind, out: &OutputConfig) -> Result<i32> {
    let backend = get_backend(kind)?;
    let text = backend.text(path, None)?;
    std::fs::write(output, &text)?;

    let result = WriteResult {
        file: output.to_string(),
        format: "txt".into(),
        message: format!("Converted {} to text", path.display()),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn convert_via_libreoffice(path: &Path, output: &str, to_format: &str, out: &OutputConfig) -> Result<i32> {
    let soffice = find_libreoffice()?;
    let output_path = Path::new(output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));

    let status = std::process::Command::new(&soffice)
        .arg("--headless")
        .arg("--convert-to")
        .arg(to_format)
        .arg("--outdir")
        .arg(output_dir)
        .arg(path)
        .status()
        .map_err(|e| OfficeError::Io(format!("failed to run LibreOffice: {}", e)))?;

    if !status.success() {
        return Err(OfficeError::Other(
            format!("LibreOffice conversion failed with exit code {:?}", status.code())
        ).into());
    }

    let result = WriteResult {
        file: output.to_string(),
        format: to_format.to_string(),
        message: format!("Converted {} to {} via LibreOffice", path.display(), to_format),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn find_libreoffice() -> Result<std::path::PathBuf> {
    // Check common locations
    if let Ok(path) = which::which("libreoffice") {
        return Ok(path);
    }
    if let Ok(path) = which::which("soffice") {
        return Ok(path);
    }

    // macOS specific
    let mac_path = Path::new("/Applications/LibreOffice.app/Contents/MacOS/soffice");
    if mac_path.exists() {
        return Ok(mac_path.to_path_buf());
    }

    Err(OfficeError::LibreOfficeNotFound.into())
}
