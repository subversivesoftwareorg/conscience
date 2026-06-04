use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConscienceError {
    #[error("GitHub authentication failed: {0}")]
    AuthError(String),

    #[error("GitHub API error: {0}")]
    GitHubApi(#[from] octocrab::Error),

    #[error("No GitHub token found. Install `gh` CLI and run `gh auth login`, or set CONSCIENCE_GITHUB_TOKEN")]
    NoToken,

    #[error("Invalid repository format '{0}'. Expected 'owner/repo'")]
    InvalidRepo(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ConscienceError>;
