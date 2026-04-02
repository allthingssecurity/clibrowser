use std::path::Path;
use std::io::Read;
use anyhow::Result;
use crate::error::OfficeError;
use crate::models::*;
use super::{FormatBackend, PageRange};

pub struct DocxBackend;

impl DocxBackend {
    fn read_xml(path: &Path, inner_path: &str) -> Result<String> {
        let file = std::fs::File::open(path).map_err(|e| {
            OfficeError::FileNotFound(format!("{}: {}", path.display(), e))
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("not a valid DOCX (ZIP): {}", e),
            }
        })?;

        let mut entry = archive.by_name(inner_path).map_err(|_| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("{} not found in archive", inner_path),
            }
        })?;

        let mut content = String::new();
        entry.read_to_string(&mut content).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("failed to read {}: {}", inner_path, e),
            }
        })?;
        Ok(content)
    }

    fn extract_text_from_xml(xml: &str) -> String {
        // Simple XML text extraction — extract content between <w:t> tags
        let mut text = String::new();
        let mut in_paragraph = false;
        let mut in_text = false;
        let mut paragraph_text = String::new();

        let mut chars = xml.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut tag = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '>' {
                        break;
                    }
                    tag.push(next);
                }

                if tag.starts_with("w:p ") || tag == "w:p" {
                    in_paragraph = true;
                    paragraph_text.clear();
                } else if tag == "/w:p" {
                    if in_paragraph && !paragraph_text.is_empty() {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(&paragraph_text);
                    }
                    in_paragraph = false;
                } else if tag.starts_with("w:t") && !tag.starts_with("w:tab") && !tag.starts_with("w:tbl") {
                    in_text = true;
                } else if tag == "/w:t" {
                    in_text = false;
                } else if tag == "w:tab/" || tag == "w:tab" {
                    paragraph_text.push('\t');
                } else if tag == "w:br/" || tag.starts_with("w:br ") {
                    paragraph_text.push('\n');
                }
            } else if in_text {
                paragraph_text.push(ch);
            }
        }
        // Flush last paragraph
        if in_paragraph && !paragraph_text.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&paragraph_text);
        }
        text
    }

    fn extract_tables_from_xml(xml: &str) -> Vec<TableData> {
        let mut tables = Vec::new();
        let mut table_idx = 0;

        // Simple state machine to extract tables
        let mut in_table = false;
        let mut in_row = false;
        let mut in_cell = false;
        let mut in_text = false;
        let mut current_rows: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut cell_text = String::new();

        let mut chars = xml.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut tag = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '>' {
                        break;
                    }
                    tag.push(next);
                }

                if tag.starts_with("w:tbl ") || tag == "w:tbl" {
                    in_table = true;
                    current_rows.clear();
                } else if tag == "/w:tbl" {
                    if in_table && !current_rows.is_empty() {
                        let cols = current_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                        tables.push(TableData {
                            index: table_idx,
                            page: None,
                            rows: current_rows.len(),
                            cols,
                            headers: None,
                            data: current_rows.clone(),
                        });
                        table_idx += 1;
                    }
                    in_table = false;
                } else if tag.starts_with("w:tr ") || tag == "w:tr" {
                    in_row = true;
                    current_row.clear();
                } else if tag == "/w:tr" {
                    if in_row {
                        current_rows.push(current_row.clone());
                    }
                    in_row = false;
                } else if tag.starts_with("w:tc ") || tag == "w:tc" {
                    in_cell = true;
                    cell_text.clear();
                } else if tag == "/w:tc" {
                    if in_cell {
                        current_row.push(cell_text.trim().to_string());
                    }
                    in_cell = false;
                } else if in_cell && tag.starts_with("w:t") && !tag.starts_with("w:tab") && !tag.starts_with("w:tbl") && !tag.starts_with("w:tc") && !tag.starts_with("w:tr") {
                    in_text = true;
                } else if tag == "/w:t" {
                    in_text = false;
                }
            } else if in_text && in_cell {
                cell_text.push(ch);
            }
        }
        tables
    }

    fn extract_hyperlinks_from_xml(doc_xml: &str, rels_xml: &str) -> Vec<LinkInfo> {
        // Build relationship map: rId -> target URL
        let mut rel_map = std::collections::HashMap::new();
        // Parse rels XML for Relationship tags
        for line in rels_xml.lines() {
            if line.contains("Relationship") && line.contains("hyperlink") {
                // Extract Id and Target attributes
                if let (Some(id), Some(target)) = (
                    extract_attr(line, "Id"),
                    extract_attr(line, "Target"),
                ) {
                    rel_map.insert(id, target);
                }
            }
        }

        let mut links = Vec::new();
        let mut idx = 0;

        // Find hyperlink references in document.xml
        let mut chars = doc_xml.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut tag = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '>' {
                        break;
                    }
                    tag.push(next);
                }

                if tag.contains("w:hyperlink") && !tag.starts_with("/") {
                    if let Some(rid) = extract_attr(&tag, "r:id") {
                        if let Some(url) = rel_map.get(&rid) {
                            // Try to get the text within this hyperlink
                            let text = extract_hyperlink_text(&mut chars);
                            links.push(LinkInfo {
                                index: idx,
                                url: url.clone(),
                                text: if text.is_empty() { None } else { Some(text) },
                                page: None,
                            });
                            idx += 1;
                        }
                    }
                }
            }
        }
        links
    }
}

fn extract_attr(s: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    if let Some(start) = s.find(&pattern) {
        let rest = &s[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn extract_hyperlink_text(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut text = String::new();
    let mut depth = 1;
    let mut in_text = false;

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '>' {
                    break;
                }
                tag.push(next);
            }
            if tag.contains("w:hyperlink") && !tag.starts_with("/") {
                depth += 1;
            } else if tag.contains("/w:hyperlink") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else if tag.starts_with("w:t") && !tag.starts_with("w:tab") && !tag.starts_with("w:tbl") {
                in_text = true;
            } else if tag == "/w:t" {
                in_text = false;
            }
        } else if in_text {
            text.push(ch);
        }
    }
    text
}

impl FormatBackend for DocxBackend {
    fn info(&self, path: &Path) -> Result<DocumentInfo> {
        let metadata = std::fs::metadata(path)?;

        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        let text = Self::extract_text_from_xml(&doc_xml);
        let word_count = text.split_whitespace().count();
        let char_count = text.len();

        // Try to read core.xml for metadata
        let mut title = None;
        let mut author = None;
        let mut subject = None;
        let mut created = None;
        let mut modified = None;

        if let Ok(core_xml) = Self::read_xml(path, "docProps/core.xml") {
            title = extract_xml_value(&core_xml, "dc:title");
            author = extract_xml_value(&core_xml, "dc:creator");
            subject = extract_xml_value(&core_xml, "dc:subject");
            created = extract_xml_value(&core_xml, "dcterms:created");
            modified = extract_xml_value(&core_xml, "dcterms:modified");
        }

        Ok(DocumentInfo {
            file: path.display().to_string(),
            format: "docx".into(),
            pages: None, // DOCX doesn't have a fixed page count without rendering
            sheets: None,
            slides: None,
            word_count: Some(word_count),
            char_count: Some(char_count),
            title,
            author,
            subject,
            creator: None,
            created,
            modified,
            file_size: metadata.len(),
        })
    }

    fn text(&self, path: &Path, _pages: Option<&PageRange>) -> Result<String> {
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        Ok(Self::extract_text_from_xml(&doc_xml))
    }

    fn pages(&self, path: &Path) -> Result<Vec<PageInfo>> {
        // DOCX doesn't have pages in the file — it's a flow document
        // Return a single "document" entry with word count
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        let text = Self::extract_text_from_xml(&doc_xml);
        Ok(vec![PageInfo {
            index: 1,
            name: "Document".into(),
            word_count: Some(text.split_whitespace().count()),
        }])
    }

    fn tables(&self, path: &Path, _pages: Option<&PageRange>) -> Result<Vec<TableData>> {
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        Ok(Self::extract_tables_from_xml(&doc_xml))
    }

    fn images(&self, path: &Path, _pages: Option<&PageRange>) -> Result<Vec<ImageInfo>> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut images = Vec::new();
        let mut idx = 0;

        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("word/media/") {
                let ext = Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let size = entry.size() as usize;
                images.push(ImageInfo {
                    index: idx,
                    page: None,
                    name: Path::new(&name).file_name().unwrap_or_default().to_string_lossy().to_string(),
                    format: ext,
                    size: Some(size),
                    saved_to: None,
                });
                idx += 1;
            }
        }
        Ok(images)
    }

    fn search(&self, path: &Path, pattern: &str, is_regex: bool, case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        let text = Self::extract_text_from_xml(&doc_xml);

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
        for (line_num, line) in text.lines().enumerate() {
            if re.is_match(line) {
                matches.push(SearchMatch {
                    page: None,
                    line: Some(line_num + 1),
                    text: line.to_string(),
                    context: None,
                });
            }
        }
        Ok(matches)
    }

    fn markdown(&self, path: &Path, _pages: Option<&PageRange>) -> Result<String> {
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        Ok(docx_xml_to_markdown(&doc_xml))
    }

    fn links(&self, path: &Path) -> Result<Vec<LinkInfo>> {
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        let rels_xml = Self::read_xml(path, "word/_rels/document.xml.rels").unwrap_or_default();
        Ok(Self::extract_hyperlinks_from_xml(&doc_xml, &rels_xml))
    }

    fn comments(&self, path: &Path) -> Result<Vec<Comment>> {
        let comments_xml = match Self::read_xml(path, "word/comments.xml") {
            Ok(xml) => xml,
            Err(_) => return Ok(Vec::new()), // No comments file
        };

        let mut comments = Vec::new();
        let mut idx = 0;

        // Simple extraction of comment text and author
        let mut in_comment = false;
        let mut comment_author = None;
        let mut comment_date = None;
        let mut comment_text = String::new();
        let mut in_text = false;

        let mut chars = comments_xml.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut tag = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '>' { break; }
                    tag.push(next);
                }

                if tag.starts_with("w:comment ") {
                    in_comment = true;
                    comment_text.clear();
                    comment_author = extract_attr(&tag, "w:author");
                    comment_date = extract_attr(&tag, "w:date");
                } else if tag == "/w:comment" {
                    if in_comment && !comment_text.is_empty() {
                        comments.push(Comment {
                            index: idx,
                            author: comment_author.take(),
                            text: comment_text.trim().to_string(),
                            page: None,
                            date: comment_date.take(),
                        });
                        idx += 1;
                    }
                    in_comment = false;
                    comment_text.clear();
                } else if in_comment && tag.starts_with("w:t") && !tag.starts_with("w:tab") {
                    in_text = true;
                } else if tag == "/w:t" {
                    in_text = false;
                }
            } else if in_text && in_comment {
                comment_text.push(ch);
            }
        }
        Ok(comments)
    }

    fn styles(&self, path: &Path) -> Result<Vec<StyleInfo>> {
        let styles_xml = Self::read_xml(path, "word/styles.xml")?;
        let mut styles = Vec::new();

        // Extract style definitions
        let mut in_style = false;
        let mut style_name = String::new();
        let mut style_type = String::new();

        for line in styles_xml.lines() {
            if line.contains("<w:style ") || line.contains("<w:style>") {
                in_style = true;
                style_type = extract_attr(line, "w:type").unwrap_or_default();
            } else if line.contains("</w:style>") {
                if in_style && !style_name.is_empty() {
                    styles.push(StyleInfo {
                        name: style_name.clone(),
                        style_type: style_type.clone(),
                        font: None,
                        size: None,
                        bold: None,
                        italic: None,
                    });
                }
                in_style = false;
                style_name.clear();
            } else if in_style && line.contains("w:name ") {
                style_name = extract_attr(line, "w:val").unwrap_or_default();
            }
        }
        Ok(styles)
    }

    fn toc(&self, path: &Path) -> Result<Vec<TocEntry>> {
        let doc_xml = Self::read_xml(path, "word/document.xml")?;
        let text = Self::extract_text_from_xml(&doc_xml);
        let mut entries = Vec::new();

        // Detect heading styles from the XML
        let mut in_paragraph = false;
        let mut current_style = String::new();
        let mut paragraph_text = String::new();
        let mut in_text = false;

        let mut chars = doc_xml.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut tag = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '>' { break; }
                    tag.push(next);
                }

                if tag.starts_with("w:p ") || tag == "w:p" {
                    in_paragraph = true;
                    current_style.clear();
                    paragraph_text.clear();
                } else if tag == "/w:p" {
                    if in_paragraph && current_style.starts_with("Heading") {
                        let level = current_style
                            .strip_prefix("Heading")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                            .unwrap_or(1);
                        if !paragraph_text.trim().is_empty() {
                            entries.push(TocEntry {
                                level,
                                text: paragraph_text.trim().to_string(),
                                page: None,
                            });
                        }
                    }
                    in_paragraph = false;
                } else if in_paragraph && tag.contains("w:pStyle") {
                    if let Some(val) = extract_attr(&tag, "w:val") {
                        current_style = val;
                    }
                } else if in_paragraph && tag.starts_with("w:t") && !tag.starts_with("w:tab") && !tag.starts_with("w:tbl") {
                    in_text = true;
                } else if tag == "/w:t" {
                    in_text = false;
                }
            } else if in_text && in_paragraph {
                paragraph_text.push(ch);
            }
        }

        let _ = text; // suppress unused warning
        Ok(entries)
    }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    if let Some(start_pos) = xml.find(&open) {
        let rest = &xml[start_pos..];
        // Find the end of the opening tag
        if let Some(gt) = rest.find('>') {
            let after_tag = &rest[gt + 1..];
            if let Some(end) = after_tag.find(&close) {
                let value = after_tag[..end].trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn docx_xml_to_markdown(xml: &str) -> String {
    let mut md = String::new();
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut current_style = String::new();
    let mut paragraph_text = String::new();
    let mut is_bold = false;
    let mut is_italic = false;

    let mut chars = xml.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '>' { break; }
                tag.push(next);
            }

            if tag.starts_with("w:p ") || tag == "w:p" {
                in_paragraph = true;
                current_style.clear();
                paragraph_text.clear();
                is_bold = false;
                is_italic = false;
            } else if tag == "/w:p" {
                if in_paragraph {
                    let trimmed = paragraph_text.trim();
                    if !trimmed.is_empty() {
                        if current_style.starts_with("Heading") {
                            let level = current_style
                                .strip_prefix("Heading")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                                .unwrap_or(1);
                            let hashes = "#".repeat(level.min(6));
                            md.push_str(&format!("{} {}\n\n", hashes, trimmed));
                        } else if current_style.contains("List") {
                            md.push_str(&format!("- {}\n", trimmed));
                        } else {
                            md.push_str(trimmed);
                            md.push_str("\n\n");
                        }
                    }
                }
                in_paragraph = false;
            } else if in_paragraph && tag.contains("w:pStyle") {
                if let Some(val) = extract_attr(&tag, "w:val") {
                    current_style = val;
                }
            } else if tag == "w:b/" || tag == "w:b" {
                is_bold = true;
            } else if tag == "w:i/" || tag == "w:i" {
                is_italic = true;
            } else if in_paragraph && tag.starts_with("w:t") && !tag.starts_with("w:tab") && !tag.starts_with("w:tbl") {
                in_text = true;
            } else if tag == "/w:t" {
                in_text = false;
            } else if tag == "/w:rPr" {
                // Reset run properties after run properties close
            } else if tag.starts_with("w:r ") || tag == "w:r" {
                is_bold = false;
                is_italic = false;
            }
        } else if in_text && in_paragraph {
            paragraph_text.push(ch);
        }
    }
    md
}
