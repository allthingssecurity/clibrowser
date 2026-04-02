pub mod pdf;
pub mod docx;
pub mod xlsx;
pub mod pptx;

use std::path::Path;
use anyhow::Result;
use crate::error::OfficeError;
use crate::format::FormatKind;
use crate::models::*;

pub struct PageRange {
    pub ranges: Vec<(usize, usize)>, // inclusive pairs
}

impl PageRange {
    pub fn parse(s: &str) -> Result<Self> {
        let mut ranges = Vec::new();
        for part in s.split(',') {
            let part = part.trim();
            if let Some((start, end)) = part.split_once('-') {
                let s: usize = start.trim().parse().map_err(|_| {
                    OfficeError::Other(format!("invalid page range: {}", part))
                })?;
                let e: usize = end.trim().parse().map_err(|_| {
                    OfficeError::Other(format!("invalid page range: {}", part))
                })?;
                ranges.push((s, e));
            } else {
                let n: usize = part.parse().map_err(|_| {
                    OfficeError::Other(format!("invalid page number: {}", part))
                })?;
                ranges.push((n, n));
            }
        }
        Ok(PageRange { ranges })
    }

    pub fn contains(&self, page: usize) -> bool {
        self.ranges.iter().any(|&(s, e)| page >= s && page <= e)
    }

    pub fn to_pages(&self) -> Vec<usize> {
        let mut pages = Vec::new();
        for &(s, e) in &self.ranges {
            for p in s..=e {
                pages.push(p);
            }
        }
        pages
    }
}

pub fn get_backend(kind: FormatKind) -> Result<Box<dyn FormatBackend>> {
    match kind {
        FormatKind::Pdf => Ok(Box::new(pdf::PdfBackend)),
        FormatKind::Docx => Ok(Box::new(docx::DocxBackend)),
        FormatKind::Xlsx => Ok(Box::new(xlsx::XlsxBackend)),
        FormatKind::Pptx => Ok(Box::new(pptx::PptxBackend)),
        FormatKind::Doc | FormatKind::Ppt => {
            Err(OfficeError::NotSupported {
                op: "direct reading".into(),
                format: kind.name().into(),
            }.into())
        }
        FormatKind::Xls => {
            // XLS is handled by calamine through the xlsx backend
            Ok(Box::new(xlsx::XlsxBackend))
        }
    }
}

pub trait FormatBackend {
    fn info(&self, path: &Path) -> Result<DocumentInfo>;
    fn text(&self, path: &Path, pages: Option<&PageRange>) -> Result<String>;
    fn pages(&self, path: &Path) -> Result<Vec<PageInfo>>;
    fn tables(&self, path: &Path, pages: Option<&PageRange>) -> Result<Vec<TableData>>;
    fn images(&self, path: &Path, pages: Option<&PageRange>) -> Result<Vec<ImageInfo>>;
    fn search(&self, path: &Path, pattern: &str, regex: bool, case_sensitive: bool) -> Result<Vec<SearchMatch>>;
    fn markdown(&self, path: &Path, pages: Option<&PageRange>) -> Result<String>;
    fn links(&self, path: &Path) -> Result<Vec<LinkInfo>>;
    fn comments(&self, path: &Path) -> Result<Vec<Comment>>;
    fn styles(&self, path: &Path) -> Result<Vec<StyleInfo>> {
        let _ = path;
        Err(OfficeError::NotSupported {
            op: "styles".into(),
            format: "this format".into(),
        }.into())
    }
    fn toc(&self, path: &Path) -> Result<Vec<TocEntry>> {
        let _ = path;
        Err(OfficeError::NotSupported {
            op: "toc".into(),
            format: "this format".into(),
        }.into())
    }
}
