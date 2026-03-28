use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clibrowser", about = "CLI browser for AI agents", version)]
pub struct Cli {
    /// Named session (default: "default")
    #[arg(long, global = true, default_value = "default")]
    pub session: String,

    /// Output JSON instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Stealth mode: use Chrome-like headers and TLS to bypass Cloudflare/bot detection
    #[arg(long, global = true)]
    pub stealth: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Fetch a URL
    Get(GetArgs),

    /// Query the page with a CSS selector
    Select(SelectArgs),

    /// Extract text content from the page
    Text(TextArgs),

    /// Extract links from the page
    Links(LinksArgs),

    /// Extract tables from the page
    Tables(TablesArgs),

    /// Follow a link by CSS selector
    Click(ClickArgs),

    /// List forms on the page
    Forms(FormsArgs),

    /// Fill form fields
    Fill(FillArgs),

    /// Submit a form
    Submit(SubmitArgs),

    /// Show response headers
    Headers(HeadersArgs),

    /// Manage cookies
    Cookies(CookiesArgs),

    /// Show session state
    Status,

    /// Manage sessions
    Session(SessionArgs),

    /// Crawl a site following links to a specified depth
    Crawl(crate::commands::crawl::CrawlArgs),

    /// Search the web (DuckDuckGo or Google)
    Search(crate::commands::search::SearchArgs),

    /// Parse an RSS/Atom feed
    Rss(crate::commands::rss::RssArgs),

    /// Discover pages via sitemap.xml
    Sitemap(crate::commands::sitemap::SitemapArgs),

    /// Convert current page to clean markdown
    Markdown(crate::commands::markdown::MarkdownArgs),

    /// Process multiple URLs from stdin
    Pipe(crate::commands::pipe::PipeArgs),

    /// Discover WebMCP tools on a page
    Webmcp(crate::commands::webmcp::WebmcpArgs),

    /// Call a WebMCP tool by name
    #[command(name = "webmcp-call")]
    WebmcpCall(crate::commands::webmcp::WebmcpCallArgs),
}

#[derive(clap::Args)]
pub struct GetArgs {
    /// URL to fetch (absolute or relative to current URL)
    pub url: String,

    /// HTTP method
    #[arg(long, default_value = "GET")]
    pub method: String,

    /// Custom header (repeatable)
    #[arg(long = "header", short = 'H')]
    pub headers: Vec<String>,

    /// Form-encoded body data
    #[arg(long)]
    pub data: Option<String>,

    /// JSON body data
    #[arg(long)]
    pub data_json: Option<String>,

    /// Don't follow redirects
    #[arg(long)]
    pub no_follow: bool,

    /// Max redirect hops
    #[arg(long, default_value = "10")]
    pub max_redirects: usize,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Override User-Agent
    #[arg(long)]
    pub user_agent: Option<String>,

    /// Stealth mode (set by global --stealth flag, not used directly)
    #[arg(skip)]
    pub stealth: bool,
}

#[derive(clap::Args)]
pub struct SelectArgs {
    /// CSS selector
    pub selector: String,

    /// Extract attribute value instead of text
    #[arg(long)]
    pub attr: Option<String>,

    /// Return inner HTML instead of text
    #[arg(long)]
    pub html: bool,

    /// Only first match
    #[arg(long)]
    pub first: bool,

    /// Max results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Get nth match (0-based)
    #[arg(long)]
    pub index: Option<usize>,
}

#[derive(clap::Args)]
pub struct TextArgs {
    /// CSS selector to extract text from (default: body)
    #[arg(long, default_value = "body")]
    pub selector: String,

    /// Aggressive whitespace normalization
    #[arg(long)]
    pub strip: bool,

    /// Truncate output to N characters
    #[arg(long)]
    pub max_length: Option<usize>,
}

#[derive(clap::Args)]
pub struct LinksArgs {
    /// Only links matching this CSS selector
    #[arg(long, default_value = "a[href]")]
    pub selector: String,

    /// Resolve all URLs to absolute
    #[arg(long)]
    pub absolute: bool,

    /// Filter by URL substring
    #[arg(long)]
    pub filter: Option<String>,
}

#[derive(clap::Args)]
pub struct TablesArgs {
    /// CSS selector for tables
    #[arg(long)]
    pub selector: Option<String>,

    /// Get nth table (0-based)
    #[arg(long)]
    pub index: Option<usize>,

    /// Use first row as column headers
    #[arg(long)]
    pub headers: bool,
}

#[derive(clap::Args)]
pub struct ClickArgs {
    /// CSS selector for the link to click
    pub selector: String,

    /// If multiple matches, click nth (0-based)
    #[arg(long)]
    pub index: Option<usize>,
}

#[derive(clap::Args)]
pub struct FormsArgs {
    /// CSS selector to filter forms
    #[arg(long)]
    pub selector: Option<String>,

    /// Show details of nth form
    #[arg(long)]
    pub index: Option<usize>,
}

#[derive(clap::Args)]
pub struct FillArgs {
    /// CSS selector for the form
    pub selector: String,

    /// Field=value pairs
    pub fields: Vec<String>,

    /// Fill nth form instead of using selector
    #[arg(long)]
    pub index: Option<usize>,
}

#[derive(clap::Args)]
pub struct SubmitArgs {
    /// CSS selector for the form
    pub selector: Option<String>,

    /// Submit nth form
    #[arg(long)]
    pub index: Option<usize>,

    /// Which submit button to click
    #[arg(long)]
    pub button: Option<String>,
}

#[derive(clap::Args)]
pub struct HeadersArgs {
    /// Get specific header value
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(clap::Args)]
pub struct CookiesArgs {
    #[command(subcommand)]
    pub action: Option<CookieAction>,

    /// Show all cookies across all domains
    #[arg(long)]
    pub all: bool,
}

#[derive(Subcommand)]
pub enum CookieAction {
    /// Set a cookie
    Set {
        name: String,
        value: String,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Clear cookies
    Clear {
        /// Domain to clear (or all if omitted)
        domain: Option<String>,
    },
}

#[derive(clap::Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub action: SessionAction,
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// List named sessions
    List,
    /// Delete a session
    Delete { name: String },
    /// Clear current session
    Clear,
}
