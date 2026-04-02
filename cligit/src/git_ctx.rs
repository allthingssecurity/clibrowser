use std::path::PathBuf;
use git2::Repository;
use crate::error::GitError;

pub struct GitContext {
    pub repo: Repository,
    pub workdir: PathBuf,
}

impl GitContext {
    pub fn open(path: Option<&str>) -> Result<Self, GitError> {
        let start = match path {
            Some(p) => PathBuf::from(p),
            None => std::env::current_dir().map_err(|e| GitError::Io(e.to_string()))?,
        };
        let repo = Repository::discover(&start).map_err(|_| GitError::NotARepo)?;
        let workdir = repo
            .workdir()
            .ok_or(GitError::BareRepo)?
            .to_path_buf();
        Ok(GitContext { repo, workdir })
    }
}
