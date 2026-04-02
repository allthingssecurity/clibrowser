use serde::Serialize;

#[derive(Serialize)]
pub struct DocumentInfo {
    pub file: String,
    pub format: String,
    pub pages: Option<usize>,
    pub sheets: Option<Vec<String>>,
    pub slides: Option<usize>,
    pub word_count: Option<usize>,
    pub char_count: Option<usize>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub file_size: u64,
}

#[derive(Serialize)]
pub struct PageInfo {
    pub index: usize,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<usize>,
}

#[derive(Serialize)]
pub struct PagesResult {
    pub count: usize,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize, Clone)]
pub struct TableData {
    pub index: usize,
    pub page: Option<usize>,
    pub rows: usize,
    pub cols: usize,
    pub headers: Option<Vec<String>>,
    pub data: Vec<Vec<String>>,
}

#[derive(Serialize)]
pub struct TablesResult {
    pub count: usize,
    pub tables: Vec<TableData>,
}

#[derive(Serialize)]
pub struct ImageInfo {
    pub index: usize,
    pub page: Option<usize>,
    pub name: String,
    pub format: String,
    pub size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_to: Option<String>,
}

#[derive(Serialize)]
pub struct ImagesResult {
    pub count: usize,
    pub images: Vec<ImageInfo>,
}

#[derive(Serialize)]
pub struct SearchMatch {
    pub page: Option<usize>,
    pub line: Option<usize>,
    pub text: String,
    pub context: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub count: usize,
    pub pattern: String,
    pub matches: Vec<SearchMatch>,
}

#[derive(Serialize)]
pub struct TextResult {
    pub text: String,
    pub length: usize,
}

#[derive(Serialize)]
pub struct MarkdownResult {
    pub markdown: String,
    pub length: usize,
}

#[derive(Serialize)]
pub struct StyleInfo {
    pub name: String,
    pub style_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
}

#[derive(Serialize)]
pub struct StylesResult {
    pub count: usize,
    pub styles: Vec<StyleInfo>,
}

#[derive(Serialize)]
pub struct Comment {
    pub index: usize,
    pub author: Option<String>,
    pub text: String,
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

#[derive(Serialize)]
pub struct CommentsResult {
    pub count: usize,
    pub comments: Vec<Comment>,
}

#[derive(Serialize)]
pub struct LinkInfo {
    pub index: usize,
    pub url: String,
    pub text: Option<String>,
    pub page: Option<usize>,
}

#[derive(Serialize)]
pub struct LinksResult {
    pub count: usize,
    pub links: Vec<LinkInfo>,
}

#[derive(Serialize)]
pub struct TocEntry {
    pub level: usize,
    pub text: String,
    pub page: Option<usize>,
}

#[derive(Serialize)]
pub struct TocResult {
    pub count: usize,
    pub entries: Vec<TocEntry>,
}

#[derive(Serialize)]
pub struct WriteResult {
    pub file: String,
    pub format: String,
    pub message: String,
}
