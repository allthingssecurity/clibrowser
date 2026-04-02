use std::path::Path;
use std::fs;
use anyhow::Result;
use crate::error::OfficeError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormatKind {
    Docx,
    Doc,
    Xlsx,
    Xls,
    Pptx,
    Ppt,
    Pdf,
}

impl FormatKind {
    pub fn name(&self) -> &'static str {
        match self {
            FormatKind::Docx => "docx",
            FormatKind::Doc => "doc",
            FormatKind::Xlsx => "xlsx",
            FormatKind::Xls => "xls",
            FormatKind::Pptx => "pptx",
            FormatKind::Ppt => "ppt",
            FormatKind::Pdf => "pdf",
        }
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self, FormatKind::Doc | FormatKind::Xls | FormatKind::Ppt)
    }
}

impl std::fmt::Display for FormatKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub fn detect_format(path: &Path, force: Option<&str>) -> Result<FormatKind> {
    // If --format flag provided, use that
    if let Some(fmt) = force {
        return match fmt.to_lowercase().as_str() {
            "docx" => Ok(FormatKind::Docx),
            "doc" => Ok(FormatKind::Doc),
            "xlsx" => Ok(FormatKind::Xlsx),
            "xls" => Ok(FormatKind::Xls),
            "pptx" => Ok(FormatKind::Pptx),
            "ppt" => Ok(FormatKind::Ppt),
            "pdf" => Ok(FormatKind::Pdf),
            _ => Err(OfficeError::UnsupportedFormat(fmt.to_string()).into()),
        };
    }

    // Check file extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "docx" => return Ok(FormatKind::Docx),
            "doc" => return Ok(FormatKind::Doc),
            "xlsx" => return Ok(FormatKind::Xlsx),
            "xls" => return Ok(FormatKind::Xls),
            "pptx" => return Ok(FormatKind::Pptx),
            "ppt" => return Ok(FormatKind::Ppt),
            "pdf" => return Ok(FormatKind::Pdf),
            _ => {}
        }
    }

    // Fall back to magic bytes
    detect_by_magic_bytes(path)
}

fn detect_by_magic_bytes(path: &Path) -> Result<FormatKind> {
    let data = fs::read(path).map_err(|e| OfficeError::Io(e.to_string()))?;
    if data.len() < 4 {
        return Err(OfficeError::UnsupportedFormat("file too small".into()).into());
    }

    // PDF: starts with %PDF
    if data.starts_with(b"%PDF") {
        return Ok(FormatKind::Pdf);
    }

    // ZIP (OOXML basis): PK\x03\x04
    if data.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
        return detect_ooxml_from_zip(path);
    }

    // OLE2/CFB (.doc/.xls/.ppt): D0 CF 11 E0 A1 B1 1A E1
    if data.len() >= 8 && data.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        // Default to doc for OLE2 without further inspection
        return Ok(FormatKind::Doc);
    }

    Err(OfficeError::UnsupportedFormat(
        format!("cannot detect format for: {}", path.display())
    ).into())
}

fn detect_ooxml_from_zip(path: &Path) -> Result<FormatKind> {
    let file = fs::File::open(path).map_err(|e| OfficeError::Io(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| OfficeError::FormatError {
            file: path.display().to_string(),
            detail: format!("invalid ZIP: {}", e),
        })?;

    // Check [Content_Types].xml for OOXML type
    if let Ok(mut entry) = archive.by_name("[Content_Types].xml") {
        use std::io::Read;
        let mut content = String::new();
        let _ = entry.read_to_string(&mut content);

        if content.contains("wordprocessingml") {
            return Ok(FormatKind::Docx);
        }
        if content.contains("spreadsheetml") {
            return Ok(FormatKind::Xlsx);
        }
        if content.contains("presentationml") {
            return Ok(FormatKind::Pptx);
        }
    }

    // Check for common paths
    for name in ["word/document.xml", "xl/workbook.xml", "ppt/presentation.xml"] {
        if archive.by_name(name).is_ok() {
            return match name {
                "word/document.xml" => Ok(FormatKind::Docx),
                "xl/workbook.xml" => Ok(FormatKind::Xlsx),
                "ppt/presentation.xml" => Ok(FormatKind::Pptx),
                _ => unreachable!(),
            };
        }
    }

    Err(OfficeError::UnsupportedFormat("ZIP file is not a recognized Office format".into()).into())
}
