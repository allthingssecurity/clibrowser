use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cliformsoffice", about = "CLI tool for MS Office & PDF files — built for AI agents", version)]
pub struct Cli {
    /// Output JSON instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Force input format (docx|doc|xlsx|xls|pptx|ppt|pdf)
    #[arg(long, global = true)]
    pub format: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    // ─── Reading / Extraction ───────────────────────────────

    /// Show document metadata (author, pages, word count, dates)
    Info(InfoArgs),

    /// Extract text content from the document
    Text(TextArgs),

    /// List pages, sheets, or slides
    Pages(PagesArgs),

    /// Extract tables as structured data
    Tables(TablesArgs),

    /// Extract embedded images
    Images(ImagesArgs),

    /// Search document content with regex
    Search(SearchArgs),

    /// Convert document to markdown
    Markdown(MarkdownArgs),

    /// List styles and formatting
    Styles(StylesArgs),

    /// Extract comments and annotations
    Comments(CommentsArgs),

    /// Extract hyperlinks
    Links(LinksArgs),

    /// Extract table of contents / outline
    Toc(TocArgs),

    // ─── Writing / Creating ─────────────────────────────────

    /// Create a new empty document
    Create(CreateArgs),

    /// Create document from markdown, text, or CSV
    Write(WriteArgs),

    /// Append text to a document
    #[command(name = "add-text")]
    AddText(AddTextArgs),

    /// Insert a table from CSV or JSON
    #[command(name = "add-table")]
    AddTable(AddTableArgs),

    /// Insert an image into a document
    #[command(name = "add-image")]
    AddImage(AddImageArgs),

    /// Convert between formats
    Convert(ConvertArgs),

    // ─── PDF-specific ───────────────────────────────────────

    /// Merge multiple PDF files
    Merge(MergeArgs),

    /// Split a PDF by page range
    Split(SplitArgs),

    /// Rotate PDF pages
    Rotate(RotateArgs),

    /// Password-protect a PDF
    Protect(ProtectArgs),
}

// ─── Read command args ───────────────────────────────────────

#[derive(clap::Args)]
pub struct InfoArgs {
    /// File path
    pub file: String,
}

#[derive(clap::Args)]
pub struct TextArgs {
    /// File path
    pub file: String,

    /// Page/sheet/slide range (e.g. "1-5", "2,4,7")
    #[arg(long)]
    pub pages: Option<String>,

    /// Aggressive whitespace normalization
    #[arg(long)]
    pub strip: bool,

    /// Truncate output to N characters
    #[arg(long)]
    pub max_length: Option<usize>,

    /// Separator between pages/sheets
    #[arg(long, default_value = "\n---\n")]
    pub separator: String,
}

#[derive(clap::Args)]
pub struct PagesArgs {
    /// File path
    pub file: String,
}

#[derive(clap::Args)]
pub struct TablesArgs {
    /// File path
    pub file: String,

    /// Extract nth table (0-based)
    #[arg(long)]
    pub index: Option<usize>,

    /// Only tables from specific pages/sheets
    #[arg(long)]
    pub page: Option<String>,

    /// Use first row as column headers
    #[arg(long)]
    pub headers: bool,

    /// Output as CSV instead of JSON/text
    #[arg(long)]
    pub csv: bool,
}

#[derive(clap::Args)]
pub struct ImagesArgs {
    /// File path
    pub file: String,

    /// Directory to save extracted images (required unless --list)
    #[arg(long)]
    pub output_dir: Option<String>,

    /// Only images from specific pages/sheets
    #[arg(long)]
    pub page: Option<String>,

    /// List images without extracting
    #[arg(long)]
    pub list: bool,
}

#[derive(clap::Args)]
pub struct SearchArgs {
    /// File path
    pub file: String,

    /// Search pattern
    pub pattern: String,

    /// Treat pattern as regex (default: literal)
    #[arg(long)]
    pub regex: bool,

    /// Case-sensitive search
    #[arg(long)]
    pub case_sensitive: bool,

    /// Limit results
    #[arg(long)]
    pub max_results: Option<usize>,

    /// Lines of context around matches
    #[arg(long, default_value = "0")]
    pub context: usize,
}

#[derive(clap::Args)]
pub struct MarkdownArgs {
    /// File path
    pub file: String,

    /// Truncate output to N characters
    #[arg(long)]
    pub max_length: Option<usize>,

    /// Page/sheet/slide range
    #[arg(long)]
    pub pages: Option<String>,
}

#[derive(clap::Args)]
pub struct StylesArgs {
    /// File path
    pub file: String,

    /// Filter by type (paragraph|character|table)
    #[arg(long, name = "type")]
    pub style_type: Option<String>,
}

#[derive(clap::Args)]
pub struct CommentsArgs {
    /// File path
    pub file: String,

    /// Filter by page/sheet
    #[arg(long)]
    pub page: Option<String>,
}

#[derive(clap::Args)]
pub struct LinksArgs {
    /// File path
    pub file: String,

    /// Filter by page/sheet
    #[arg(long)]
    pub page: Option<String>,
}

#[derive(clap::Args)]
pub struct TocArgs {
    /// File path
    pub file: String,

    /// Max heading depth
    #[arg(long)]
    pub depth: Option<usize>,
}

// ─── Write command args ──────────────────────────────────────

#[derive(clap::Args)]
pub struct CreateArgs {
    /// Output file path (.docx, .xlsx, .pptx, .pdf)
    pub file: String,

    /// Document title
    #[arg(long)]
    pub title: Option<String>,

    /// Author name
    #[arg(long)]
    pub author: Option<String>,
}

#[derive(clap::Args)]
pub struct WriteArgs {
    /// Output file path
    pub file: String,

    /// Source file (markdown, txt, CSV, JSON)
    #[arg(long)]
    pub from: String,

    /// Document title
    #[arg(long)]
    pub title: Option<String>,
}

#[derive(clap::Args)]
pub struct AddTextArgs {
    /// Target document
    pub file: String,

    /// Text content to add
    pub content: String,

    /// Target page/sheet/slide (default: last)
    #[arg(long)]
    pub page: Option<usize>,

    /// Apply named style (Heading1, Normal, etc.)
    #[arg(long)]
    pub style: Option<String>,

    /// Bold text
    #[arg(long)]
    pub bold: bool,

    /// Italic text
    #[arg(long)]
    pub italic: bool,
}

#[derive(clap::Args)]
pub struct AddTableArgs {
    /// Target document
    pub file: String,

    /// CSV or JSON file with table data
    #[arg(long)]
    pub from: String,

    /// Target page/sheet/slide
    #[arg(long)]
    pub page: Option<usize>,

    /// First row of input is headers
    #[arg(long)]
    pub headers: bool,
}

#[derive(clap::Args)]
pub struct AddImageArgs {
    /// Target document
    pub file: String,

    /// Image file path
    pub image: String,

    /// Target page/sheet/slide
    #[arg(long)]
    pub page: Option<usize>,

    /// Image width in pixels
    #[arg(long)]
    pub width: Option<u32>,

    /// Image height in pixels
    #[arg(long)]
    pub height: Option<u32>,
}

#[derive(clap::Args)]
pub struct ConvertArgs {
    /// Input file
    pub file: String,

    /// Target format (pdf|docx|xlsx|pptx|md|html|csv)
    #[arg(long)]
    pub to: String,

    /// Output file path (default: same name, new extension)
    #[arg(short, long)]
    pub output: Option<String>,
}

// ─── PDF-specific args ───────────────────────────────────────

#[derive(clap::Args)]
pub struct MergeArgs {
    /// PDF files to merge
    pub files: Vec<String>,

    /// Output file
    #[arg(short, long)]
    pub output: String,
}

#[derive(clap::Args)]
pub struct SplitArgs {
    /// Input PDF
    pub file: String,

    /// Page range to extract (e.g. "1-5", "2,4,7-10")
    #[arg(long)]
    pub pages: String,

    /// Output file
    #[arg(short, long)]
    pub output: String,
}

#[derive(clap::Args)]
pub struct RotateArgs {
    /// Input PDF
    pub file: String,

    /// Pages to rotate (default: all)
    #[arg(long)]
    pub pages: Option<String>,

    /// Rotation angle: 90, 180, 270
    #[arg(long)]
    pub angle: i32,

    /// Output file (default: in-place)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ProtectArgs {
    /// Input PDF
    pub file: String,

    /// Password to set
    #[arg(long)]
    pub password: String,

    /// Output file (default: in-place)
    #[arg(short, long)]
    pub output: Option<String>,
}
