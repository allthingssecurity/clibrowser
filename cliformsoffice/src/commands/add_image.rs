use std::path::Path;
use anyhow::Result;
use crate::cli::AddImageArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: AddImageArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file.clone()).into());
    }

    let image_path = Path::new(&args.image);
    if !image_path.exists() {
        return Err(OfficeError::FileNotFound(args.image.clone()).into());
    }

    let kind = detect_format(path, format_override)?;

    match kind {
        crate::format::FormatKind::Docx => add_image_docx(path, image_path, &args, out),
        _ => Err(OfficeError::NotSupported {
            op: "add-image".into(),
            format: kind.name().into(),
        }.into()),
    }
}

fn add_image_docx(path: &Path, image_path: &Path, _args: &AddImageArgs, out: &OutputConfig) -> Result<i32> {
    let data = std::fs::read(path)?;
    let image_data = std::fs::read(image_path)?;

    let mut doc = docx_rs::read_docx(&data).map_err(|e| {
        OfficeError::FormatError {
            file: path.display().to_string(),
            detail: format!("failed to parse DOCX: {}", e),
        }
    })?;

    // Determine image type from extension
    let ext = image_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let pic = docx_rs::Pic::new(&image_data);
    doc = doc.add_paragraph(
        docx_rs::Paragraph::new().add_run(
            docx_rs::Run::new().add_image(pic)
        )
    );

    let file = std::fs::File::create(path)?;
    doc.build().pack(file)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "docx".into(),
        message: format!("Image {} added to document", image_path.display()),
    };
    let _ = ext;
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
