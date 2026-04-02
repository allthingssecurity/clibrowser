use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a git repository (or any parent up to mount point)")]
    NotARepo,

    #[error("Cannot operate on bare repository")]
    BareRepo,

    #[error("Reference not found: {0}")]
    RefNotFound(String),

    #[error("File not found: {0} (ref: {1})")]
    FileNotFound(String, String),

    #[error("Merge conflicts in {0} file(s)")]
    MergeConflict(usize),

    #[error("Working tree has uncommitted changes")]
    DirtyWorkTree,

    #[error("Authentication required for {0}")]
    AuthRequired(String),

    #[error("Remote error: {0}")]
    RemoteError(String),

    #[error("git2: {0}")]
    Git2(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("{0}")]
    Other(String),
}

impl GitError {
    pub fn exit_code(&self) -> i32 {
        match self {
            GitError::NotARepo | GitError::BareRepo => 1,
            GitError::RefNotFound(_)
            | GitError::FileNotFound(_, _)
            | GitError::MergeConflict(_)
            | GitError::DirtyWorkTree
            | GitError::Git2(_)
            | GitError::Other(_) => 2,
            GitError::Io(_) => 3,
            GitError::AuthRequired(_) | GitError::RemoteError(_) => 4,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            GitError::NotARepo => "not_a_repo",
            GitError::BareRepo => "bare_repo",
            GitError::RefNotFound(_) => "ref_not_found",
            GitError::FileNotFound(_, _) => "file_not_found",
            GitError::MergeConflict(_) => "merge_conflict",
            GitError::DirtyWorkTree => "dirty_worktree",
            GitError::AuthRequired(_) => "auth_required",
            GitError::RemoteError(_) => "remote_error",
            GitError::Git2(_) => "git2_error",
            GitError::Io(_) => "io_error",
            GitError::Other(_) => "error",
        }
    }
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError::Git2(e.message().to_string())
    }
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::Io(e.to_string())
    }
}
