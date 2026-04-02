use std::path::Path;
use std::io::Read;
use anyhow::Result;
use crate::cli::ImagesArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::formats::{get_backend, PageRange};
use crate::models::ImagesResult;
use crate::output::OutputConfig;

pub fn execute(args: ImagesArgs, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(OfficeError::FileNotFound(args.file.clone()).into());
    }

    let kind = detect_format(path, format_override)?;
    let backend = get_backend(kind)?;

    let page_range = args.page.as_ref().map(|p| PageRange::parse(p)).transpose()?;
    let mut images = backend.images(path, page_range.as_ref())?;

    // Extract images to disk if --output-dir specified
    if let Some(ref output_dir) = args.output_dir {
        if !args.list {
            let out_path = Path::new(output_dir);
            std::fs::create_dir_all(out_path)?;

            // Re-open archive and extract media files
            let media_prefix = match kind {
                crate::format::FormatKind::Docx => "word/media/",
                crate::format::FormatKind::Xlsx => "xl/media/",
                crate::format::FormatKind::Pptx => "ppt/media/",
                _ => "",
            };

            if !media_prefix.is_empty() {
                let file = std::fs::File::open(path)?;
                let mut archive = zip::ZipArchive::new(file)?;

                for image in &mut images {
                    let inner = format!("{}{}", media_prefix, image.name);
                    if let Ok(mut entry) = archive.by_name(&inner) {
                        let dest = out_path.join(&image.name);
                        let mut data = Vec::new();
                        entry.read_to_end(&mut data)?;
                        std::fs::write(&dest, &data)?;
                        image.saved_to = Some(dest.display().to_string());
                    }
                }
            }
        }
    }

    if out.json {
        let result = ImagesResult {
            count: images.len(),
            images,
        };
        out.print_json(&result);
    } else {
        for img in &images {
            let page_str = img.page.map(|p| format!(" (page {})", p)).unwrap_or_default();
            let size_str = img.size.map(|s| format!(" [{}b]", s)).unwrap_or_default();
            let saved = img.saved_to.as_ref().map(|s| format!(" -> {}", s)).unwrap_or_default();
            out.print_human(&format!("[{}] {}.{}{}{}{}", img.index, img.name, img.format, page_str, size_str, saved));
        }
        out.print_human(&format!("\n{} images found", images.len()));
    }
    Ok(0)
}
