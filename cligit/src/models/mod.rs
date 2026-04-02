use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub author: AuthorInfo,
    pub date: String,
    pub parents: Vec<String>,
    pub is_merge: bool,
}

#[derive(Serialize, Clone)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Serialize, Clone)]
pub struct DiffLine {
    pub op: String,
    pub content: String,
}

#[derive(Serialize, Clone)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize, Clone)]
pub struct DiffFile {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Serialize, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct StatusCounts {
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
    pub conflicts: usize,
}

#[derive(Serialize)]
pub struct StatusResult {
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<StatusEntry>,
    pub modified: Vec<StatusEntry>,
    pub untracked: Vec<StatusEntry>,
    pub conflicts: Vec<StatusEntry>,
    pub clean: bool,
    pub counts: StatusCounts,
}

#[derive(Serialize)]
pub struct LogResult {
    pub count: usize,
    pub commits: Vec<CommitInfo>,
}

#[derive(Serialize)]
pub struct DiffResult {
    pub stats: DiffStats,
    pub files: Vec<DiffFile>,
}

#[derive(Serialize)]
pub struct ShowResult {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub author: AuthorInfo,
    pub committer: AuthorInfo,
    pub date: String,
    pub parents: Vec<String>,
    pub stats: DiffStats,
    pub files: Vec<DiffFile>,
}

#[derive(Serialize)]
pub struct BlameLineInfo {
    pub line_no: usize,
    pub sha: String,
    pub author: String,
    pub date: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct BlameResult {
    pub file: String,
    pub lines: Vec<BlameLineInfo>,
}

#[derive(Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_message: Option<String>,
}

#[derive(Serialize)]
pub struct BranchesResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    pub count: usize,
    pub branches: Vec<BranchInfo>,
}

#[derive(Serialize)]
pub struct TagInfo {
    pub name: String,
    pub sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagger: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub is_annotated: bool,
}

#[derive(Serialize)]
pub struct TagsResult {
    pub count: usize,
    pub tags: Vec<TagInfo>,
}

#[derive(Serialize)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    pub sha: String,
}

#[derive(Serialize)]
pub struct StashesResult {
    pub count: usize,
    pub stashes: Vec<StashInfo>,
}

#[derive(Serialize)]
pub struct RemoteInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_url: Option<String>,
}

#[derive(Serialize)]
pub struct RemotesResult {
    pub count: usize,
    pub remotes: Vec<RemoteInfo>,
}

#[derive(Serialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub mode: String,
}

#[derive(Serialize)]
pub struct FilesResult {
    pub count: usize,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize)]
pub struct CatResult {
    pub file: String,
    pub size: usize,
    pub lines: usize,
    pub content: String,
}

#[derive(Serialize, Clone)]
pub struct HistoryCommit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: String,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Serialize)]
pub struct HistoryResult {
    pub file: String,
    pub count: usize,
    pub commits: Vec<HistoryCommit>,
}

#[derive(Serialize)]
pub struct ContributorInfo {
    pub name: String,
    pub email: String,
    pub commits: usize,
    pub additions: usize,
    pub deletions: usize,
    pub first_commit: String,
    pub last_commit: String,
}

#[derive(Serialize)]
pub struct ContributorsResult {
    pub count: usize,
    pub contributors: Vec<ContributorInfo>,
}

#[derive(Serialize)]
pub struct SearchMatch {
    pub file: String,
    pub line_no: usize,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_before: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_after: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub pattern: String,
    pub mode: String,
    pub count: usize,
    pub matches: serde_json::Value,
}

#[derive(Serialize)]
pub struct FindResult {
    pub pattern: String,
    pub count: usize,
    pub files: Vec<String>,
}

#[derive(Serialize)]
pub struct WriteResult {
    pub message: String,
}

#[derive(Serialize)]
pub struct SummaryResult {
    pub repo_name: String,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub clean: bool,
    pub status_counts: StatusCounts,
    pub recent_commits: Vec<CommitInfo>,
    pub branch_count: usize,
    pub tag_count: usize,
    pub remote_count: usize,
    pub tracked_files: usize,
    pub contributors: usize,
}

#[derive(Serialize)]
pub struct ChangesResult {
    pub from_ref: String,
    pub to_ref: String,
    pub commit_count: usize,
    pub authors: Vec<String>,
    pub stats: DiffStats,
    pub files: Vec<DiffFile>,
}

#[derive(Serialize)]
pub struct PrDiffResult {
    pub base: String,
    pub head: String,
    pub merge_base: String,
    pub commits: usize,
    pub stats: DiffStats,
    pub files: Vec<DiffFile>,
}

#[derive(Serialize)]
pub struct ConflictEntry {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ours_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theirs_ref: Option<String>,
}

#[derive(Serialize)]
pub struct ConflictsResult {
    pub count: usize,
    pub conflicts: Vec<ConflictEntry>,
}

#[derive(Serialize)]
pub struct ConfigResult {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Serialize)]
pub struct ConfigListResult {
    pub entries: Vec<ConfigResult>,
}
