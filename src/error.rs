use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("No page loaded. Run `clibrowser get <url>` first.")]
    NoPageLoaded,

    #[error("No current URL. Run `clibrowser get <url>` first.")]
    NoCurrentUrl,

    #[error("Invalid CSS selector: {0}")]
    InvalidSelector(String),

    #[error("No element found matching selector: {0}")]
    NoMatch(String),

    #[error("Index {index} out of range (found {count} matches)")]
    IndexOutOfRange { index: usize, count: usize },

    #[error("No form found matching selector: {0}")]
    NoFormFound(String),

    #[error("No form found at index {0}")]
    NoFormAtIndex(usize),

    #[error("HTTP error {status}: {url}")]
    HttpStatus { status: u16, url: String },

    #[error("Network error: {0}")]
    Network(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("{0}")]
    Other(String),
}

impl BrowserError {
    pub fn exit_code(&self) -> i32 {
        match self {
            BrowserError::HttpStatus { .. } => 2,
            BrowserError::Network(_) => 3,
            _ => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            BrowserError::NoPageLoaded => "no_page",
            BrowserError::NoCurrentUrl => "no_url",
            BrowserError::InvalidSelector(_) => "invalid_selector",
            BrowserError::NoMatch(_) => "no_match",
            BrowserError::IndexOutOfRange { .. } => "index_out_of_range",
            BrowserError::NoFormFound(_) => "no_form",
            BrowserError::NoFormAtIndex(_) => "no_form",
            BrowserError::HttpStatus { .. } => "http_error",
            BrowserError::Network(_) => "network_error",
            BrowserError::SessionNotFound(_) => "session_not_found",
            BrowserError::Other(_) => "error",
        }
    }
}
