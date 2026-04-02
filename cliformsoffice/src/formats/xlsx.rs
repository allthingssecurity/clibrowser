use std::path::Path;
use anyhow::Result;
use calamine::{Reader, open_workbook_auto, Data};
use crate::error::OfficeError;
use crate::models::*;
use super::{FormatBackend, PageRange};

pub struct XlsxBackend;

impl XlsxBackend {
    fn open_workbook(path: &Path) -> Result<calamine::Sheets<std::io::BufReader<std::fs::File>>> {
        open_workbook_auto(path).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("failed to open: {}", e),
            }.into()
        })
    }

    fn data_to_string(data: &Data) -> String {
        match data {
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                if *f == f.floor() && f.abs() < 1e15 {
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Data::String(s) => s.clone(),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(dt) => format!("{}", dt),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
            Data::Error(e) => format!("{:?}", e),
            Data::Empty => String::new(),
        }
    }
}

impl FormatBackend for XlsxBackend {
    fn info(&self, path: &Path) -> Result<DocumentInfo> {
        let mut wb = Self::open_workbook(path)?;
        let metadata = std::fs::metadata(path)?;
        let sheets: Vec<String> = wb.sheet_names().to_vec();
        let sheet_count = sheets.len();

        // Count total cells/words across sheets
        let mut word_count = 0;
        let mut char_count = 0;
        for name in &sheets {
            if let Ok(range) = wb.worksheet_range(name) {
                for row in range.rows() {
                    for cell in row {
                        let s = Self::data_to_string(cell);
                        word_count += s.split_whitespace().count();
                        char_count += s.len();
                    }
                }
            }
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("xlsx");
        Ok(DocumentInfo {
            file: path.display().to_string(),
            format: ext.to_string(),
            pages: None,
            sheets: Some(sheets),
            slides: None,
            word_count: Some(word_count),
            char_count: Some(char_count),
            title: None,
            author: None,
            subject: None,
            creator: None,
            created: None,
            modified: None,
            file_size: metadata.len(),
        })
    }

    fn text(&self, path: &Path, pages: Option<&PageRange>) -> Result<String> {
        let mut wb = Self::open_workbook(path)?;
        let sheets = wb.sheet_names().to_vec();
        let mut result = Vec::new();

        for (i, name) in sheets.iter().enumerate() {
            let sheet_num = i + 1;
            if let Some(range) = pages {
                if !range.contains(sheet_num) {
                    continue;
                }
            }

            if let Ok(data) = wb.worksheet_range(name) {
                let mut sheet_text = format!("=== Sheet: {} ===\n", name);
                for row in data.rows() {
                    let cells: Vec<String> = row.iter()
                        .map(|c| Self::data_to_string(c))
                        .collect();
                    sheet_text.push_str(&cells.join("\t"));
                    sheet_text.push('\n');
                }
                result.push(sheet_text);
            }
        }
        Ok(result.join("\n---\n"))
    }

    fn pages(&self, path: &Path) -> Result<Vec<PageInfo>> {
        let mut wb = Self::open_workbook(path)?;
        let sheets = wb.sheet_names().to_vec();
        let mut pages = Vec::new();

        for (i, name) in sheets.iter().enumerate() {
            let mut wc = 0;
            if let Ok(data) = wb.worksheet_range(name) {
                for row in data.rows() {
                    for cell in row {
                        wc += Self::data_to_string(cell).split_whitespace().count();
                    }
                }
            }
            pages.push(PageInfo {
                index: i + 1,
                name: name.clone(),
                word_count: Some(wc),
            });
        }
        Ok(pages)
    }

    fn tables(&self, path: &Path, pages: Option<&PageRange>) -> Result<Vec<TableData>> {
        let mut wb = Self::open_workbook(path)?;
        let sheets = wb.sheet_names().to_vec();
        let mut tables = Vec::new();

        for (i, name) in sheets.iter().enumerate() {
            let sheet_num = i + 1;
            if let Some(range) = pages {
                if !range.contains(sheet_num) {
                    continue;
                }
            }

            if let Ok(data) = wb.worksheet_range(name) {
                let mut rows_data: Vec<Vec<String>> = Vec::new();
                let mut max_cols = 0;

                for row in data.rows() {
                    let cells: Vec<String> = row.iter()
                        .map(|c| Self::data_to_string(c))
                        .collect();
                    max_cols = max_cols.max(cells.len());
                    rows_data.push(cells);
                }

                if !rows_data.is_empty() {
                    tables.push(TableData {
                        index: i,
                        page: Some(sheet_num),
                        rows: rows_data.len(),
                        cols: max_cols,
                        headers: None,
                        data: rows_data,
                    });
                }
            }
        }
        Ok(tables)
    }

    fn images(&self, path: &Path, _pages: Option<&PageRange>) -> Result<Vec<ImageInfo>> {
        // Check for images in the ZIP archive
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut images = Vec::new();
        let mut idx = 0;

        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("xl/media/") {
                let ext = Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                images.push(ImageInfo {
                    index: idx,
                    page: None,
                    name: Path::new(&name).file_name().unwrap_or_default().to_string_lossy().to_string(),
                    format: ext,
                    size: Some(entry.size() as usize),
                    saved_to: None,
                });
                idx += 1;
            }
        }
        Ok(images)
    }

    fn search(&self, path: &Path, pattern: &str, is_regex: bool, case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let mut wb = Self::open_workbook(path)?;
        let sheets = wb.sheet_names().to_vec();

        let re = if is_regex {
            let mut builder = regex::RegexBuilder::new(pattern);
            builder.case_insensitive(!case_sensitive);
            builder.build().map_err(|e| OfficeError::Other(format!("invalid regex: {}", e)))?
        } else {
            let escaped = regex::escape(pattern);
            let mut builder = regex::RegexBuilder::new(&escaped);
            builder.case_insensitive(!case_sensitive);
            builder.build().map_err(|e| OfficeError::Other(format!("regex error: {}", e)))?
        };

        let mut matches = Vec::new();
        for (sheet_idx, name) in sheets.iter().enumerate() {
            if let Ok(data) = wb.worksheet_range(name) {
                for (row_idx, row) in data.rows().enumerate() {
                    for cell in row {
                        let val = Self::data_to_string(cell);
                        if re.is_match(&val) {
                            matches.push(SearchMatch {
                                page: Some(sheet_idx + 1),
                                line: Some(row_idx + 1),
                                text: val,
                                context: Some(format!("Sheet: {}", name)),
                            });
                        }
                    }
                }
            }
        }
        Ok(matches)
    }

    fn markdown(&self, path: &Path, pages: Option<&PageRange>) -> Result<String> {
        let mut wb = Self::open_workbook(path)?;
        let sheets = wb.sheet_names().to_vec();
        let mut md = String::new();

        for (i, name) in sheets.iter().enumerate() {
            let sheet_num = i + 1;
            if let Some(range) = pages {
                if !range.contains(sheet_num) {
                    continue;
                }
            }

            if let Ok(data) = wb.worksheet_range(name) {
                md.push_str(&format!("## {}\n\n", name));

                let rows: Vec<Vec<String>> = data.rows()
                    .map(|row| row.iter().map(|c| Self::data_to_string(c)).collect())
                    .collect();

                if rows.is_empty() {
                    continue;
                }

                let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                if max_cols == 0 {
                    continue;
                }

                // First row as header
                let header = &rows[0];
                md.push('|');
                for j in 0..max_cols {
                    md.push_str(&format!(" {} |", header.get(j).map(|s| s.as_str()).unwrap_or("")));
                }
                md.push('\n');

                // Separator
                md.push('|');
                for _ in 0..max_cols {
                    md.push_str(" --- |");
                }
                md.push('\n');

                // Data rows
                for row in rows.iter().skip(1) {
                    md.push('|');
                    for j in 0..max_cols {
                        md.push_str(&format!(" {} |", row.get(j).map(|s| s.as_str()).unwrap_or("")));
                    }
                    md.push('\n');
                }
                md.push('\n');
            }
        }
        Ok(md)
    }

    fn links(&self, _path: &Path) -> Result<Vec<LinkInfo>> {
        // TODO: Extract hyperlinks from cells
        Ok(Vec::new())
    }

    fn comments(&self, _path: &Path) -> Result<Vec<Comment>> {
        // TODO: Extract cell comments
        Ok(Vec::new())
    }
}
