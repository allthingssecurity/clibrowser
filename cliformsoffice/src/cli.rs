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

    // ─── v0.2: Agent-Centric Features ──────────────────────

    /// One-call document digest (title, outline, counts, previews)
    Summary(SummaryArgs),

    /// Compare two documents structurally
    Diff(DiffArgs),

    /// Extract structured data by named patterns
    Extract(ExtractArgs),

    /// Find and replace text preserving formatting
    Replace(ReplaceArgs),

    /// Process multiple files from stdin (one path per line)
    Pipe(PipeArgs),

    /// Process multiple files by glob pattern
    Batch(BatchArgs),

    /// Read/write individual spreadsheet cells
    Cells(CellsArgs),

    /// Infer spreadsheet column types and statistics
    Schema(SchemaArgs),

    /// Insert markdown-formatted section into document
    #[command(name = "add-section")]
    AddSection(AddSectionArgs),

    /// SQL-like query on spreadsheet data
    Query(QueryArgs),

    /// Fill {{placeholder}} templates with JSON data
    #[command(name = "fill-template")]
    FillTemplate(FillTemplateArgs),

    /// Detect and replace PII (SSN, email, phone, etc.)
    Redact(RedactArgs),

    /// Run validation rules on a document
    Validate(ValidateArgs),

    /// Extract spreadsheet formulas
    Formulas(FormulasArgs),

    /// Quantitative content analysis (reading level, word frequency)
    Stats(StatsArgs),

    /// Add text or image watermark
    Watermark(WatermarkArgs),

    /// Read/write document headers and footers
    #[command(name = "headers-footers")]
    HeadersFooters(HeadersFootersArgs),

    /// Remove specific content (pages, comments, images, etc.)
    Remove(RemoveArgs),
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

// ─── v0.2: Agent-Centric arg structs ─────────────────────────

#[derive(clap::Args)]
pub struct SummaryArgs {
    pub file: String,
    #[arg(long, default_value = "200")]
    pub max_preview: usize,
}

#[derive(clap::Args)]
pub struct DiffArgs {
    pub file1: String,
    pub file2: String,
    #[arg(long)]
    pub ignore_whitespace: bool,
}

#[derive(clap::Args)]
pub struct ExtractArgs {
    pub file: String,
    #[arg(long = "pattern", short = 'p')]
    pub patterns: Vec<String>,
    #[arg(long)]
    pub all: bool,
}

#[derive(clap::Args)]
pub struct ReplaceArgs {
    pub file: String,
    #[arg(long)]
    pub find: String,
    #[arg(long, name = "with")]
    pub replace_with: String,
    #[arg(long)]
    pub regex: bool,
    #[arg(long)]
    pub all: bool,
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct PipeArgs {
    pub subcommand: String,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(clap::Args)]
pub struct BatchArgs {
    pub subcommand: String,
    #[arg(long)]
    pub glob: String,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(clap::Args)]
pub struct CellsArgs {
    pub file: String,
    #[arg(long)]
    pub range: Option<String>,
    #[arg(long)]
    pub sheet: Option<String>,
    #[arg(long = "set")]
    pub sets: Vec<String>,
}

#[derive(clap::Args)]
pub struct SchemaArgs {
    pub file: String,
    #[arg(long)]
    pub sheet: Option<String>,
    #[arg(long, default_value = "100")]
    pub sample: usize,
}

#[derive(clap::Args)]
pub struct AddSectionArgs {
    pub file: String,
    pub content: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub after: Option<String>,
}

#[derive(clap::Args)]
pub struct QueryArgs {
    pub file: String,
    pub sql: String,
    #[arg(long)]
    pub csv: bool,
}

#[derive(clap::Args)]
pub struct FillTemplateArgs {
    pub file: String,
    #[arg(long)]
    pub data: String,
    #[arg(short, long)]
    pub output: String,
}

#[derive(clap::Args)]
pub struct RedactArgs {
    pub file: String,
    #[arg(long)]
    pub email: bool,
    #[arg(long)]
    pub phone: bool,
    #[arg(long)]
    pub ssn: bool,
    #[arg(long)]
    pub credit_card: bool,
    #[arg(long = "pattern")]
    pub patterns: Vec<String>,
    #[arg(short, long)]
    pub output: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(clap::Args)]
pub struct ValidateArgs {
    pub file: String,
    #[arg(long)]
    pub required_fields: Option<String>,
    #[arg(long)]
    pub max_pages: Option<usize>,
    #[arg(long)]
    pub no_empty_cells: bool,
    #[arg(long)]
    pub check_links: bool,
}

#[derive(clap::Args)]
pub struct FormulasArgs {
    pub file: String,
    #[arg(long)]
    pub sheet: Option<String>,
    #[arg(long)]
    pub cell: Option<String>,
}

#[derive(clap::Args)]
pub struct StatsArgs {
    pub file: String,
    #[arg(long)]
    pub detailed: bool,
}

#[derive(clap::Args)]
pub struct WatermarkArgs {
    pub file: String,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long, default_value = "0.3")]
    pub opacity: f64,
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct HeadersFootersArgs {
    pub file: String,
    #[arg(long)]
    pub set_header: Option<String>,
    #[arg(long)]
    pub set_footer: Option<String>,
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct RemoveArgs {
    pub file: String,
    #[arg(long)]
    pub pages: Option<String>,
    #[arg(long)]
    pub comments: bool,
    #[arg(long)]
    pub images: bool,
    #[arg(long)]
    pub links: bool,
    #[arg(short, long)]
    pub output: Option<String>,
}
