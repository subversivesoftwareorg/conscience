use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoSummary {
    pub owner: String,
    pub repo: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub commits: Vec<CommitSummary>,
    pub pull_requests: Vec<PullRequestSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitSummary {
    pub sha: String,
    pub author: String,
    pub message: String,
    pub date: DateTime<Utc>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestSummary {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub state: String,
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub review_comments: u64,
    pub time_to_merge_hours: Option<f64>,
}
