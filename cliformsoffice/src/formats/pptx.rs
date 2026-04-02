use std::path::Path;
use std::io::Read;
use anyhow::Result;
use crate::error::OfficeError;
use crate::models::*;
use super::{FormatBackend, PageRange};

pub struct PptxBackend;

impl PptxBackend {
    fn read_xml(path: &Path, inner_path: &str) -> Result<String> {
        let file = std::fs::File::open(path).map_err(|e| {
            OfficeError::FileNotFound(format!("{}: {}", path.display(), e))
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("not a valid PPTX (ZIP): {}", e),
            }
        })?;

        let mut entry = archive.by_name(inner_path).map_err(|_| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("{} not found in archive", inner_path),
            }
        })?;

        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        Ok(content)
    }

    fn get_slide_count(path: &Path) -> Result<usize> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut count = 0;
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") && !name.contains("Layout") {
                count += 1;
            }
        }
        Ok(count)
    }

    fn extract_text_from_xml(xml: &str) -> String {
        // Extract text from <a:t> tags (DrawingML text)
        let mut text = String::new();
        let mut in_text = false;
        let mut in_paragraph = false;
        let mut paragraph_text = String::new();

        let mut chars = xml.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '<' {
                let mut tag = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '>' { break; }
                    tag.push(next);
                }

                if tag.starts_with("a:p ") || tag == "a:p" {
                    in_paragraph = true;
                    paragraph_text.clear();
                } else if tag == "/a:p" {
                    if in_paragraph && !paragraph_text.is_empty() {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(&paragraph_text);
                    }
                    in_paragraph = false;
                } else if tag.starts_with("a:t") && tag.len() <= 5 {
                    in_text = true;
                } else if tag == "/a:t" {
                    in_text = false;
                }
            } else if in_text {
                paragraph_text.push(ch);
            }
        }
        if in_paragraph && !paragraph_text.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&paragraph_text);
        }
        text
    }

    fn extract_tables_from_slide(xml: &str, slide_num: usize) -> Vec<TableData> {
        let mut tables = Vec::new();
        let mut table_idx = 0;

        // Look for <a:tbl> elements
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
                    if next == '>' { break; }
                    tag.push(next);
                }

                if tag.starts_with("a:tbl ") || tag == "a:tbl" {
                    in_table = true;
                    current_rows.clear();
                } else if tag == "/a:tbl" {
                    if in_table && !current_rows.is_empty() {
                        let cols = current_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                        tables.push(TableData {
                            index: table_idx,
                            page: Some(slide_num),
                            rows: current_rows.len(),
                            cols,
                            headers: None,
                            data: current_rows.clone(),
                        });
                        table_idx += 1;
                    }
                    in_table = false;
                } else if in_table && (tag.starts_with("a:tr ") || tag == "a:tr") {
                    in_row = true;
                    current_row.clear();
                } else if tag == "/a:tr" {
                    if in_row {
                        current_rows.push(current_row.clone());
                    }
                    in_row = false;
                } else if in_row && (tag.starts_with("a:tc ") || tag == "a:tc") {
                    in_cell = true;
                    cell_text.clear();
                } else if tag == "/a:tc" {
                    if in_cell {
                        current_row.push(cell_text.trim().to_string());
                    }
                    in_cell = false;
                } else if in_cell && tag.starts_with("a:t") && tag.len() <= 5 {
                    in_text = true;
                } else if tag == "/a:t" {
                    in_text = false;
                }
            } else if in_text && in_cell {
                cell_text.push(ch);
            }
        }
        tables
    }
}

impl FormatBackend for PptxBackend {
    fn info(&self, path: &Path) -> Result<DocumentInfo> {
        let metadata = std::fs::metadata(path)?;
        let slide_count = Self::get_slide_count(path)?;

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

        // Count words across all slides
        let mut word_count = 0;
        let mut char_count = 0;
        for i in 1..=slide_count {
            if let Ok(slide_xml) = Self::read_xml(path, &format!("ppt/slides/slide{}.xml", i)) {
                let text = Self::extract_text_from_xml(&slide_xml);
                word_count += text.split_whitespace().count();
                char_count += text.len();
            }
        }

        Ok(DocumentInfo {
            file: path.display().to_string(),
            format: "pptx".into(),
            pages: None,
            sheets: None,
            slides: Some(slide_count),
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

    fn text(&self, path: &Path, pages: Option<&PageRange>) -> Result<String> {
        let slide_count = Self::get_slide_count(path)?;
        let mut result = Vec::new();

        for i in 1..=slide_count {
            if let Some(range) = pages {
                if !range.contains(i) {
                    continue;
                }
            }

            if let Ok(slide_xml) = Self::read_xml(path, &format!("ppt/slides/slide{}.xml", i)) {
                let text = Self::extract_text_from_xml(&slide_xml);
                if !text.is_empty() {
                    result.push(format!("=== Slide {} ===\n{}", i, text));
                }
            }
        }
        Ok(result.join("\n---\n"))
    }

    fn pages(&self, path: &Path) -> Result<Vec<PageInfo>> {
        let slide_count = Self::get_slide_count(path)?;
        let mut pages = Vec::new();

        for i in 1..=slide_count {
            let mut wc = None;
            if let Ok(slide_xml) = Self::read_xml(path, &format!("ppt/slides/slide{}.xml", i)) {
                let text = Self::extract_text_from_xml(&slide_xml);
                wc = Some(text.split_whitespace().count());
            }
            pages.push(PageInfo {
                index: i,
                name: format!("Slide {}", i),
                word_count: wc,
            });
        }
        Ok(pages)
    }

    fn tables(&self, path: &Path, pages: Option<&PageRange>) -> Result<Vec<TableData>> {
        let slide_count = Self::get_slide_count(path)?;
        let mut all_tables = Vec::new();

        for i in 1..=slide_count {
            if let Some(range) = pages {
                if !range.contains(i) {
                    continue;
                }
            }

            if let Ok(slide_xml) = Self::read_xml(path, &format!("ppt/slides/slide{}.xml", i)) {
                let mut tables = Self::extract_tables_from_slide(&slide_xml, i);
                all_tables.append(&mut tables);
            }
        }

        // Re-index
        for (idx, table) in all_tables.iter_mut().enumerate() {
            table.index = idx;
        }
        Ok(all_tables)
    }

    fn images(&self, path: &Path, _pages: Option<&PageRange>) -> Result<Vec<ImageInfo>> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut images = Vec::new();
        let mut idx = 0;

        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("ppt/media/") {
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
        let slide_count = Self::get_slide_count(path)?;

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
        for i in 1..=slide_count {
            if let Ok(slide_xml) = Self::read_xml(path, &format!("ppt/slides/slide{}.xml", i)) {
                let text = Self::extract_text_from_xml(&slide_xml);
                for (line_num, line) in text.lines().enumerate() {
                    if re.is_match(line) {
                        matches.push(SearchMatch {
                            page: Some(i),
                            line: Some(line_num + 1),
                            text: line.to_string(),
                            context: None,
                        });
                    }
                }
            }
        }
        Ok(matches)
    }

    fn markdown(&self, path: &Path, pages: Option<&PageRange>) -> Result<String> {
        let slide_count = Self::get_slide_count(path)?;
        let mut md = String::new();

        for i in 1..=slide_count {
            if let Some(range) = pages {
                if !range.contains(i) {
                    continue;
                }
            }

            if let Ok(slide_xml) = Self::read_xml(path, &format!("ppt/slides/slide{}.xml", i)) {
                let text = Self::extract_text_from_xml(&slide_xml);
                let lines: Vec<&str> = text.lines().collect();

                md.push_str(&format!("## Slide {}\n\n", i));
                if let Some(first) = lines.first() {
                    md.push_str(&format!("### {}\n\n", first.trim()));
                    for line in lines.iter().skip(1) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            md.push_str(&format!("- {}\n", trimmed));
                        }
                    }
                }
                md.push_str("\n---\n\n");
            }
        }
        Ok(md)
    }

    fn links(&self, path: &Path) -> Result<Vec<LinkInfo>> {
        let slide_count = Self::get_slide_count(path)?;
        let mut links = Vec::new();
        let mut idx = 0;

        for i in 1..=slide_count {
            // Read slide rels for hyperlinks
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", i);
            if let Ok(rels_xml) = Self::read_xml(path, &rels_path) {
                for line in rels_xml.lines() {
                    if line.contains("hyperlink") {
                        if let Some(target) = extract_attr_simple(line, "Target") {
                            links.push(LinkInfo {
                                index: idx,
                                url: target,
                                text: None,
                                page: Some(i),
                            });
                            idx += 1;
                        }
                    }
                }
            }
        }
        Ok(links)
    }

    fn comments(&self, path: &Path) -> Result<Vec<Comment>> {
        // PPTX comments are in ppt/comments/comment*.xml
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut comments = Vec::new();
        let mut idx = 0;

        for i in 0..archive.len() {
            let entry_name = archive.by_index(i)?.name().to_string();
            if entry_name.starts_with("ppt/comments/") && entry_name.ends_with(".xml") {
                if let Ok(xml) = Self::read_xml(path, &entry_name) {
                    // Extract slide number from the file name pattern
                    let slide_num = entry_name
                        .strip_prefix("ppt/comments/comment")
                        .and_then(|s| s.strip_suffix(".xml"))
                        .and_then(|s| s.parse::<usize>().ok());

                    let text = extract_text_tags(&xml, "p:text");
                    let author = extract_attr_from_content(&xml, "p:cm", "authorId");
                    for t in text {
                        comments.push(Comment {
                            index: idx,
                            author: author.clone(),
                            text: t,
                            page: slide_num,
                            date: None,
                        });
                        idx += 1;
                    }
                }
            }
        }
        Ok(comments)
    }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    if let Some(start_pos) = xml.find(&open) {
        let rest = &xml[start_pos..];
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

fn extract_attr_simple(s: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    if let Some(start) = s.find(&pattern) {
        let rest = &s[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn extract_text_tags(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start) = xml[search_from..].find(&open) {
        let abs_start = search_from + start;
        let rest = &xml[abs_start..];
        if let Some(gt) = rest.find('>') {
            let after = &rest[gt + 1..];
            if let Some(end) = after.find(&close) {
                results.push(after[..end].trim().to_string());
                search_from = abs_start + gt + 1 + end + close.len();
                continue;
            }
        }
        break;
    }
    results
}

fn extract_attr_from_content(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let open = format!("<{} ", tag);
    if let Some(start) = xml.find(&open) {
        let rest = &xml[start..];
        if let Some(end) = rest.find('>') {
            let tag_str = &rest[..end];
            return extract_attr_simple(tag_str, attr);
        }
    }
    None
}
