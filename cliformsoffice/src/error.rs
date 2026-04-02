use thiserror::Error;

#[derive(Error, Debug)]
pub enum OfficeError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Format error in {file}: {detail}")]
    FormatError { file: String, detail: String },

    #[error("Page/sheet/slide {0} out of range")]
    PageOutOfRange(usize),

    #[error("LibreOffice not found. Install it for legacy format (.doc/.ppt) support.")]
    LibreOfficeNotFound,

    #[error("Password required to open {0}")]
    PasswordRequired(String),

    #[error("Operation '{op}' not supported for {format} files")]
    NotSupported { op: String, format: String },

    #[error("IO error: {0}")]
    Io(String),

    #[error("{0}")]
    Other(String),
}

impl OfficeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            OfficeError::UnsupportedFormat(_) | OfficeError::FormatError { .. } => 2,
            OfficeError::Io(_) | OfficeError::FileNotFound(_) => 3,
            _ => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            OfficeError::UnsupportedFormat(_) => "unsupported_format",
            OfficeError::FileNotFound(_) => "file_not_found",
            OfficeError::FormatError { .. } => "format_error",
            OfficeError::PageOutOfRange(_) => "page_out_of_range",
            OfficeError::LibreOfficeNotFound => "libreoffice_not_found",
            OfficeError::PasswordRequired(_) => "password_required",
            OfficeError::NotSupported { .. } => "not_supported",
            OfficeError::Io(_) => "io_error",
            OfficeError::Other(_) => "error",
        }
    }
}
