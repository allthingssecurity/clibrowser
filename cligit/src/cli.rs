use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cligit", about = "Git CLI for AI agents", version)]
pub struct Cli {
    #[arg(long, global = true, help = "Output JSON")]
    pub json: bool,

    #[arg(long, global = true, help = "Suppress human output")]
    pub quiet: bool,

    #[arg(short = 'C', long, global = true, help = "Run as if started in <path>")]
    pub directory: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    // Read commands
    /// Show working tree status
    Status(StatusArgs),
    /// Show commit log
    Log(LogArgs),
    /// Show diff of changes
    Diff(DiffArgs),
    /// Show a commit's details and diff
    Show(ShowArgs),
    /// Show per-line blame for a file
    Blame(BlameArgs),
    /// List branches
    Branches(BranchesArgs),
    /// List tags
    Tags(TagsArgs),
    /// List stashes
    Stashes(StashesArgs),
    /// List remotes
    Remotes(RemotesArgs),

    // File commands
    /// List tracked files
    Files(FilesArgs),
    /// Show file contents at a ref
    Cat(CatArgs),
    /// Show commit history for a file
    History(HistoryArgs),
    /// List contributors with stats
    Contributors(ContributorsArgs),
    /// Search file contents or commits
    Search(SearchArgs),
    /// Find files by glob pattern
    Find(FindArgs),

    // Write commands
    /// Stage files
    Stage(StageArgs),
    /// Unstage files
    Unstage(UnstageArgs),
    /// Create a commit
    Commit(CommitArgs),
    /// Create or delete a branch
    Branch(BranchArgs),
    /// Checkout a branch or file
    Checkout(CheckoutArgs),
    /// Create a tag
    Tag(TagArgs),
    /// Stash changes
    Stash(StashArgs),
    /// Merge a branch
    Merge(MergeArgs),
    /// Rebase onto a branch
    Rebase(RebaseArgs),
    /// Cherry-pick a commit
    CherryPick(CherryPickArgs),
    /// Reset to a ref
    Reset(ResetArgs),

    // Remote commands
    /// Fetch from remote
    Fetch(FetchArgs),
    /// Pull from remote
    Pull(PullArgs),
    /// Push to remote
    Push(PushArgs),
    /// Clone a repository
    Clone(CloneArgs),

    // Agent commands
    /// Repository summary for AI context
    Summary(SummaryArgs),
    /// Show changes between two refs
    Changes(ChangesArgs),
    /// Show PR-style diff between branches
    PrDiff(PrDiffArgs),
    /// Show merge conflicts
    Conflicts(ConflictsArgs),
    /// Get or set config values
    Config(ConfigArgs),

    // Analysis commands (v0.2)
    /// Deep repository analysis (architecture, tech stack, key files)
    Analyze(AnalyzeArgs),
    /// Smart filtered file tree
    Tree(TreeArgs),
    /// Parse dependencies from manifest files
    Deps(DepsArgs),
    /// Language breakdown with line counts
    Languages(LanguagesArgs),
    /// LLM-ready repo context packet
    Context(ContextArgs),
}

// --- Read command args ---

#[derive(Parser)]
pub struct StatusArgs {
    #[arg(long, help = "Show short format")]
    pub short: bool,
}

#[derive(Parser)]
pub struct LogArgs {
    #[arg(long, default_value = "20", help = "Max commits to show")]
    pub max: usize,
    #[arg(long, help = "Start from this ref")]
    pub r#ref: Option<String>,
    #[arg(long, help = "Filter by author")]
    pub author: Option<String>,
    #[arg(long, help = "Filter by message pattern")]
    pub grep: Option<String>,
    #[arg(long, help = "Filter by file path")]
    pub path: Option<String>,
    #[arg(long, help = "After date (ISO 8601)")]
    pub since: Option<String>,
    #[arg(long, help = "Before date (ISO 8601)")]
    pub until: Option<String>,
}

#[derive(Parser)]
pub struct DiffArgs {
    #[arg(help = "First ref (or single ref to diff against worktree)")]
    pub from: Option<String>,
    #[arg(help = "Second ref")]
    pub to: Option<String>,
    #[arg(long, help = "Show staged changes")]
    pub staged: bool,
    #[arg(long, help = "Filter by file path")]
    pub path: Option<String>,
    #[arg(long, help = "Stats only, no hunks")]
    pub stat: bool,
}

#[derive(Parser)]
pub struct ShowArgs {
    #[arg(default_value = "HEAD", help = "Commit ref to show")]
    pub r#ref: String,
    #[arg(long, help = "Stats only, no hunks")]
    pub stat: bool,
}

#[derive(Parser)]
pub struct BlameArgs {
    #[arg(help = "File path to blame")]
    pub file: String,
    #[arg(long, help = "Ref to blame at")]
    pub r#ref: Option<String>,
}

#[derive(Parser)]
pub struct BranchesArgs {
    #[arg(long, help = "Show remote branches")]
    pub remote: bool,
    #[arg(long, help = "Show all branches")]
    pub all: bool,
}

#[derive(Parser)]
pub struct TagsArgs {}

#[derive(Parser)]
pub struct StashesArgs {}

#[derive(Parser)]
pub struct RemotesArgs {}

// --- File command args ---

#[derive(Parser)]
pub struct FilesArgs {
    #[arg(long, default_value = "HEAD", help = "Ref to list files from")]
    pub r#ref: String,
    #[arg(long, help = "Filter by glob pattern")]
    pub pattern: Option<String>,
}

#[derive(Parser)]
pub struct CatArgs {
    #[arg(help = "File path")]
    pub file: String,
    #[arg(long, default_value = "HEAD", help = "Ref to read from")]
    pub r#ref: String,
}

#[derive(Parser)]
pub struct HistoryArgs {
    #[arg(help = "File path")]
    pub file: String,
    #[arg(long, default_value = "20", help = "Max commits")]
    pub max: usize,
}

#[derive(Parser)]
pub struct ContributorsArgs {
    #[arg(long, default_value = "50", help = "Max contributors")]
    pub max: usize,
}

#[derive(Parser)]
pub struct SearchArgs {
    #[arg(help = "Search pattern (regex)")]
    pub pattern: String,
    #[arg(long, default_value = "grep", help = "Mode: grep or pickaxe")]
    pub mode: String,
    #[arg(long, default_value = "HEAD", help = "Ref to search")]
    pub r#ref: String,
    #[arg(long, default_value = "50", help = "Max results")]
    pub max: usize,
}

#[derive(Parser)]
pub struct FindArgs {
    #[arg(help = "Glob pattern")]
    pub pattern: String,
    #[arg(long, default_value = "HEAD", help = "Ref to search")]
    pub r#ref: String,
}

// --- Write command args ---

#[derive(Parser)]
pub struct StageArgs {
    #[arg(required = true, help = "Files to stage")]
    pub files: Vec<String>,
}

#[derive(Parser)]
pub struct UnstageArgs {
    #[arg(required = true, help = "Files to unstage")]
    pub files: Vec<String>,
}

#[derive(Parser)]
pub struct CommitArgs {
    #[arg(short, long, help = "Commit message")]
    pub message: String,
    #[arg(long, help = "Stage all modified files before commit")]
    pub all: bool,
}

#[derive(Parser)]
pub struct BranchArgs {
    #[arg(help = "Branch name")]
    pub name: String,
    #[arg(long, help = "Delete the branch")]
    pub delete: bool,
    #[arg(long, help = "Start point ref")]
    pub from: Option<String>,
}

#[derive(Parser)]
pub struct CheckoutArgs {
    #[arg(help = "Branch name or file path")]
    pub target: String,
    #[arg(long, help = "Create new branch")]
    pub create: bool,
    #[arg(long, help = "Checkout file from this ref")]
    pub from: Option<String>,
    #[arg(long, help = "Treat target as a file path")]
    pub file: bool,
}

#[derive(Parser)]
pub struct TagArgs {
    #[arg(help = "Tag name")]
    pub name: String,
    #[arg(short, long, help = "Tag message (creates annotated tag)")]
    pub message: Option<String>,
    #[arg(long, help = "Ref to tag")]
    pub r#ref: Option<String>,
    #[arg(long, help = "Delete the tag")]
    pub delete: bool,
}

#[derive(Parser)]
pub struct StashArgs {
    #[arg(default_value = "push", help = "Action: push, pop, apply, drop, list")]
    pub action: String,
    #[arg(short, long, help = "Stash message")]
    pub message: Option<String>,
    #[arg(long, help = "Stash index (for pop/apply/drop)")]
    pub index: Option<usize>,
}

#[derive(Parser)]
pub struct MergeArgs {
    #[arg(help = "Branch to merge")]
    pub branch: String,
    #[arg(long, help = "Do not fast-forward")]
    pub no_ff: bool,
    #[arg(short, long, help = "Merge commit message")]
    pub message: Option<String>,
}

#[derive(Parser)]
pub struct RebaseArgs {
    #[arg(help = "Branch to rebase onto")]
    pub onto: String,
    #[arg(long, help = "Abort in-progress rebase")]
    pub abort: bool,
    #[arg(long, help = "Continue rebase after resolving")]
    pub r#continue: bool,
}

#[derive(Parser)]
pub struct CherryPickArgs {
    #[arg(help = "Commit to cherry-pick")]
    pub commit: String,
    #[arg(long, help = "Do not commit")]
    pub no_commit: bool,
}

#[derive(Parser)]
pub struct ResetArgs {
    #[arg(default_value = "HEAD", help = "Ref to reset to")]
    pub r#ref: String,
    #[arg(long, default_value = "mixed", help = "Mode: soft, mixed, hard")]
    pub mode: String,
}

// --- Remote command args ---

#[derive(Parser)]
pub struct FetchArgs {
    #[arg(default_value = "origin", help = "Remote name")]
    pub remote: String,
    #[arg(long, help = "Fetch all remotes")]
    pub all: bool,
    #[arg(long, help = "Prune deleted remote branches")]
    pub prune: bool,
}

#[derive(Parser)]
pub struct PullArgs {
    #[arg(help = "Remote name")]
    pub remote: Option<String>,
    #[arg(help = "Branch name")]
    pub branch: Option<String>,
    #[arg(long, help = "Rebase instead of merge")]
    pub rebase: bool,
}

#[derive(Parser)]
pub struct PushArgs {
    #[arg(help = "Remote name")]
    pub remote: Option<String>,
    #[arg(help = "Branch name")]
    pub branch: Option<String>,
    #[arg(long, help = "Force push")]
    pub force: bool,
    #[arg(long, short, help = "Set upstream")]
    pub set_upstream: bool,
    #[arg(long, help = "Push tags")]
    pub tags: bool,
}

#[derive(Parser)]
pub struct CloneArgs {
    #[arg(help = "Repository URL")]
    pub url: String,
    #[arg(help = "Target directory")]
    pub directory: Option<String>,
    #[arg(long, help = "Branch to checkout")]
    pub branch: Option<String>,
    #[arg(long, help = "Shallow clone depth")]
    pub depth: Option<u32>,
}

// --- Agent command args ---

#[derive(Parser)]
pub struct SummaryArgs {}

#[derive(Parser)]
pub struct ChangesArgs {
    #[arg(help = "From ref")]
    pub from: String,
    #[arg(help = "To ref (default HEAD)")]
    pub to: Option<String>,
}

#[derive(Parser)]
pub struct PrDiffArgs {
    #[arg(help = "Base branch")]
    pub base: String,
    #[arg(help = "Head branch (default HEAD)")]
    pub head: Option<String>,
}

#[derive(Parser)]
pub struct ConflictsArgs {}

#[derive(Parser)]
pub struct ConfigArgs {
    #[arg(help = "Config key")]
    pub key: Option<String>,
    #[arg(help = "Config value (set mode)")]
    pub value: Option<String>,
    #[arg(long, help = "List all config")]
    pub list: bool,
    #[arg(long, help = "Scope: local, global, system")]
    pub scope: Option<String>,
}

// v0.2 analysis args

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// Analysis depth (quick or deep)
    #[arg(long, default_value = "deep")]
    pub depth: String,
}

#[derive(clap::Args)]
pub struct TreeArgs {
    /// Max directory depth
    #[arg(long)]
    pub depth: Option<usize>,
    /// Show file sizes
    #[arg(long)]
    pub sizes: bool,
    /// Don't filter out noise directories
    #[arg(long)]
    pub no_filter: bool,
    /// Glob pattern to filter
    #[arg(long)]
    pub pattern: Option<String>,
}

#[derive(clap::Args)]
pub struct DepsArgs {}

#[derive(clap::Args)]
pub struct LanguagesArgs {}

#[derive(clap::Args)]
pub struct ContextArgs {
    /// Max output length in characters
    #[arg(long, default_value = "8000")]
    pub max_length: usize,
    /// Include README content
    #[arg(long, default_value = "true")]
    pub include_readme: bool,
    /// Include file tree
    #[arg(long, default_value = "true")]
    pub include_tree: bool,
    /// Include dependencies
    #[arg(long, default_value = "true")]
    pub include_deps: bool,
}
