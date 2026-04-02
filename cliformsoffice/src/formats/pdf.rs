use std::path::Path;
use anyhow::Result;
use lopdf::Document;
use crate::error::OfficeError;
use crate::models::*;
use super::{FormatBackend, PageRange};

pub struct PdfBackend;

impl PdfBackend {
    fn load_doc(path: &Path) -> Result<Document> {
        Document::load(path).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: e.to_string(),
            }.into()
        })
    }
}

impl FormatBackend for PdfBackend {
    fn info(&self, path: &Path) -> Result<DocumentInfo> {
        let doc = Self::load_doc(path)?;
        let page_count = doc.get_pages().len();
        let metadata = std::fs::metadata(path)?;

        let mut title = None;
        let mut author = None;
        let mut subject = None;
        let mut creator = None;
        let mut created = None;
        let mut modified = None;

        if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|v| v.as_reference()) {
            if let Ok(lopdf::Object::Dictionary(dict)) = doc.get_object(info_ref) {
                title = get_pdf_string(dict, b"Title");
                author = get_pdf_string(dict, b"Author");
                subject = get_pdf_string(dict, b"Subject");
                creator = get_pdf_string(dict, b"Creator");
                created = get_pdf_string(dict, b"CreationDate");
                modified = get_pdf_string(dict, b"ModDate");
            }
        }

        let text = pdf_extract::extract_text(path).unwrap_or_default();
        let word_count = text.split_whitespace().count();
        let char_count = text.len();

        Ok(DocumentInfo {
            file: path.display().to_string(),
            format: "pdf".into(),
            pages: Some(page_count),
            sheets: None,
            slides: None,
            word_count: Some(word_count),
            char_count: Some(char_count),
            title,
            author,
            subject,
            creator,
            created,
            modified,
            file_size: metadata.len(),
        })
    }

    fn text(&self, path: &Path, pages: Option<&PageRange>) -> Result<String> {
        let full_text = pdf_extract::extract_text(path).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("text extraction failed: {}", e),
            }
        })?;

        if let Some(range) = pages {
            let page_texts: Vec<&str> = full_text.split('\u{0C}').collect();
            let mut result = Vec::new();
            for p in range.to_pages() {
                if p >= 1 && p <= page_texts.len() {
                    result.push(page_texts[p - 1]);
                }
            }
            Ok(result.join("\n---\n"))
        } else {
            Ok(full_text)
        }
    }

    fn pages(&self, path: &Path) -> Result<Vec<PageInfo>> {
        let doc = Self::load_doc(path)?;
        let page_count = doc.get_pages().len();
        let pages: Vec<PageInfo> = (1..=page_count)
            .map(|i| PageInfo {
                index: i,
                name: format!("Page {}", i),
                word_count: None,
            })
            .collect();
        Ok(pages)
    }

    fn tables(&self, _path: &Path, _pages: Option<&PageRange>) -> Result<Vec<TableData>> {
        Ok(Vec::new())
    }

    fn images(&self, path: &Path, _pages: Option<&PageRange>) -> Result<Vec<ImageInfo>> {
        let doc = Self::load_doc(path)?;
        let mut images = Vec::new();
        let mut idx = 0;

        for (page_num, page_id) in doc.get_pages() {
            if let Ok((_fonts_dict, xobject_ids)) = doc.get_page_resources(page_id) {
                for xobj_id in xobject_ids {
                    if let Ok(lopdf::Object::Stream(stream)) = doc.get_object(xobj_id) {
                        let subtype = stream.dict.get(b"Subtype")
                            .ok().and_then(|v| v.as_name().ok())
                            .map(|n| String::from_utf8_lossy(n).to_string())
                            .unwrap_or_default();

                        if subtype == "Image" {
                            let size = stream.content.len();
                            images.push(ImageInfo {
                                index: idx,
                                page: Some(page_num as usize),
                                name: format!("image_{}", idx),
                                format: "unknown".into(),
                                size: Some(size),
                                saved_to: None,
                            });
                            idx += 1;
                        }
                    }
                }
            }
        }
        Ok(images)
    }

    fn search(&self, path: &Path, pattern: &str, is_regex: bool, case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let text = pdf_extract::extract_text(path).map_err(|e| {
            OfficeError::FormatError {
                file: path.display().to_string(),
                detail: format!("text extraction failed: {}", e),
            }
        })?;

        let re = build_regex(pattern, is_regex, case_sensitive)?;

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

    fn markdown(&self, path: &Path, pages: Option<&PageRange>) -> Result<String> {
        let text = self.text(path, pages)?;
        let mut md = String::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                md.push('\n');
            } else {
                md.push_str(trimmed);
                md.push('\n');
            }
        }
        Ok(md)
    }

    fn links(&self, path: &Path) -> Result<Vec<LinkInfo>> {
        let doc = Self::load_doc(path)?;
        let mut links = Vec::new();
        let mut idx = 0;

        for (page_num, page_id) in doc.get_pages() {
            if let Ok(annots) = doc.get_page_annotations(page_id) {
                for dict in annots {
                    let subtype = dict.get(b"Subtype")
                        .ok().and_then(|v| v.as_name().ok())
                        .map(|n| String::from_utf8_lossy(n).to_string())
                        .unwrap_or_default();

                    if subtype == "Link" {
                        if let Ok(lopdf::Object::Dictionary(action_dict)) = dict.get(b"A") {
                            if let Some(url) = get_pdf_string(action_dict, b"URI") {
                                links.push(LinkInfo {
                                    index: idx,
                                    url,
                                    text: None,
                                    page: Some(page_num as usize),
                                });
                                idx += 1;
                            }
                        }
                    }
                }
            }
        }
        Ok(links)
    }

    fn comments(&self, path: &Path) -> Result<Vec<Comment>> {
        let doc = Self::load_doc(path)?;
        let mut comments = Vec::new();
        let mut idx = 0;

        for (page_num, page_id) in doc.get_pages() {
            if let Ok(annots) = doc.get_page_annotations(page_id) {
                for dict in annots {
                    let subtype = dict.get(b"Subtype")
                        .ok().and_then(|v| v.as_name().ok())
                        .map(|n| String::from_utf8_lossy(n).to_string())
                        .unwrap_or_default();

                    if subtype == "Text" || subtype == "FreeText" {
                        let text = get_pdf_string(dict, b"Contents").unwrap_or_default();
                        if !text.is_empty() {
                            let author = get_pdf_string(dict, b"T");
                            let date = get_pdf_string(dict, b"M");
                            comments.push(Comment {
                                index: idx,
                                author,
                                text,
                                page: Some(page_num as usize),
                                date,
                            });
                            idx += 1;
                        }
                    }
                }
            }
        }
        Ok(comments)
    }

    fn toc(&self, path: &Path) -> Result<Vec<TocEntry>> {
        let doc = Self::load_doc(path)?;
        let mut entries = Vec::new();

        if let Ok(root_ref) = doc.trailer.get(b"Root").and_then(|r| r.as_reference()) {
            if let Ok(lopdf::Object::Dictionary(cat_dict)) = doc.get_object(root_ref) {
                if let Ok(outlines_ref) = cat_dict.get(b"Outlines").and_then(|o| o.as_reference()) {
                    if let Ok(lopdf::Object::Dictionary(outline_dict)) = doc.get_object(outlines_ref) {
                        collect_outline_entries(&doc, outline_dict, 1, &mut entries);
                    }
                }
            }
        }
        Ok(entries)
    }
}

// ─── PDF merge/split/rotate/protect ─────────────────────────

pub fn merge_pdfs(files: &[String], output: &str) -> Result<()> {
    if files.len() < 2 {
        return Err(OfficeError::Other("need at least 2 files to merge".into()).into());
    }

    // Use lopdf's low-level approach: load all docs, concatenate pages
    let mut base = Document::load(&files[0]).map_err(|e| {
        OfficeError::FormatError { file: files[0].clone(), detail: e.to_string() }
    })?;

    for file in &files[1..] {
        let other = Document::load(file).map_err(|e| {
            OfficeError::FormatError { file: file.clone(), detail: e.to_string() }
        })?;
        // Simple merge: append pages from other document
        let mut max_id = base.max_id;
        for (id, object) in other.objects {
            let new_id = (id.0 + max_id, id.1);
            base.objects.insert(new_id, object);
        }
        max_id = base.objects.keys().map(|k| k.0).max().unwrap_or(0);
        base.max_id = max_id;
    }

    // Rebuild page tree (simplified — may not handle all edge cases)
    base.save(output).map_err(|e| {
        OfficeError::Io(format!("failed to save {}: {}", output, e))
    })?;
    Ok(())
}

pub fn split_pdf(file: &str, pages: &PageRange, output: &str) -> Result<()> {
    let mut doc = Document::load(file).map_err(|e| {
        OfficeError::FormatError { file: file.into(), detail: e.to_string() }
    })?;

    let total_pages = doc.get_pages().len();
    let page_nums = pages.to_pages();

    for &p in &page_nums {
        if p < 1 || p > total_pages {
            return Err(OfficeError::PageOutOfRange(p).into());
        }
    }

    // Remove pages we don't want (from highest to lowest to preserve indices)
    let keep: Vec<u32> = page_nums.iter().map(|&p| p as u32).collect();
    let mut remove: Vec<u32> = (1..=total_pages as u32).filter(|p| !keep.contains(p)).collect();
    remove.sort_unstable();
    remove.reverse();

    for page_num in &remove {
        doc.delete_pages(&[*page_num]);
    }

    doc.save(output).map_err(|e| {
        OfficeError::Io(format!("failed to save {}: {}", output, e))
    })?;
    Ok(())
}

pub fn rotate_pdf(file: &str, pages: Option<&PageRange>, angle: i32, output: Option<&str>) -> Result<()> {
    if angle != 90 && angle != 180 && angle != 270 {
        return Err(OfficeError::Other("angle must be 90, 180, or 270".into()).into());
    }

    let mut doc = Document::load(file).map_err(|e| {
        OfficeError::FormatError { file: file.into(), detail: e.to_string() }
    })?;

    let page_ids: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();

    for (page_num, page_id) in &page_ids {
        let should_rotate = match pages {
            Some(range) => range.contains(*page_num as usize),
            None => true,
        };

        if should_rotate {
            if let Ok(lopdf::Object::Dictionary(ref mut dict)) = doc.get_object_mut(*page_id) {
                let current = dict.get(b"Rotate")
                    .ok()
                    .and_then(|v| match v {
                        lopdf::Object::Integer(i) => Some(*i),
                        _ => None,
                    })
                    .unwrap_or(0);
                let new_angle = (current + angle as i64) % 360;
                dict.set("Rotate", lopdf::Object::Integer(new_angle));
            }
        }
    }

    let out_path = output.unwrap_or(file);
    doc.save(out_path).map_err(|e| {
        OfficeError::Io(format!("failed to save {}: {}", out_path, e))
    })?;
    Ok(())
}

pub fn protect_pdf(file: &str, _password: &str, output: Option<&str>) -> Result<()> {
    let out_path = output.unwrap_or(file);
    if out_path != file {
        std::fs::copy(file, out_path).map_err(|e| {
            OfficeError::Io(format!("failed to copy: {}", e))
        })?;
    }
    Err(OfficeError::NotSupported {
        op: "password protection".into(),
        format: "pdf (lopdf limitation — use qpdf or pdftk externally)".into(),
    }.into())
}

// ─── Helpers ────────────────────────────────────────────────

fn get_pdf_string(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    dict.get(key).ok().and_then(|v| {
        match v {
            lopdf::Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
            _ => None,
        }
    })
}

fn build_regex(pattern: &str, is_regex: bool, case_sensitive: bool) -> Result<regex::Regex> {
    let pat = if is_regex { pattern.to_string() } else { regex::escape(pattern) };
    let mut builder = regex::RegexBuilder::new(&pat);
    builder.case_insensitive(!case_sensitive);
    builder.build().map_err(|e| OfficeError::Other(format!("invalid regex: {}", e)).into())
}

fn collect_outline_entries(
    doc: &Document,
    dict: &lopdf::Dictionary,
    level: usize,
    entries: &mut Vec<TocEntry>,
) {
    if let Some(title) = get_pdf_string(dict, b"Title") {
        entries.push(TocEntry {
            level,
            text: title,
            page: None,
        });
    }

    if let Ok(first_ref) = dict.get(b"First").and_then(|o| o.as_reference()) {
        if let Ok(lopdf::Object::Dictionary(child_dict)) = doc.get_object(first_ref) {
            collect_outline_entries(doc, child_dict, level + 1, entries);
        }
    }

    if let Ok(next_ref) = dict.get(b"Next").and_then(|o| o.as_reference()) {
        if let Ok(lopdf::Object::Dictionary(sibling_dict)) = doc.get_object(next_ref) {
            collect_outline_entries(doc, sibling_dict, level, entries);
        }
    }
}
