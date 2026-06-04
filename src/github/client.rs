use crate::error::{ConscienceError, Result};
use crate::github::models::{CommitSummary, PullRequestSummary, RepoSummary};
use chrono::{DateTime, Utc};
use octocrab::Octocrab;

pub struct GitHubClient {
    client: Octocrab,
}

impl GitHubClient {
    pub fn new(client: Octocrab) -> Self {
        Self { client }
    }

    pub fn parse_repo(repo: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(ConscienceError::InvalidRepo(repo.to_string()));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    pub async fn fetch_summary(
        &self,
        owner: &str,
        repo: &str,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<RepoSummary> {
        let commits = self.fetch_commits(owner, repo, since, until).await?;
        let pull_requests = self.fetch_pull_requests(owner, repo, since, until).await?;

        Ok(RepoSummary {
            owner: owner.to_string(),
            repo: repo.to_string(),
            period_start: since,
            period_end: until,
            commits,
            pull_requests,
        })
    }

    async fn fetch_commits(
        &self,
        owner: &str,
        repo: &str,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<CommitSummary>> {
        let mut commits = Vec::new();
        let mut page = 1u32;

        loop {
            let response = self
                .client
                .repos(owner, repo)
                .list_commits()
                .since(since)
                .until(until)
                .per_page(100)
                .page(page)
                .send()
                .await?;

            if response.items.is_empty() {
                break;
            }

            for commit in &response.items {
                let author = commit
                    .author
                    .as_ref()
                    .map(|a| a.login.clone())
                    .unwrap_or_else(|| {
                        commit
                            .commit
                            .author
                            .as_ref()
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "unknown".to_string())
                    });

                let date = commit
                    .commit
                    .author
                    .as_ref()
                    .and_then(|a| a.date)
                    .unwrap_or(since);

                commits.push(CommitSummary {
                    sha: commit.sha.clone(),
                    author,
                    message: commit.commit.message.clone(),
                    date,
                    additions: None,
                    deletions: None,
                });
            }

            if response.next.is_none() {
                break;
            }
            page += 1;
        }

        Ok(commits)
    }

    async fn fetch_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        since: DateTime<Utc>,
        _until: DateTime<Utc>,
    ) -> Result<Vec<PullRequestSummary>> {
        let mut prs = Vec::new();
        let mut page = 1u32;

        loop {
            let response = self
                .client
                .pulls(owner, repo)
                .list()
                .state(octocrab::params::State::All)
                .sort(octocrab::params::pulls::Sort::Updated)
                .direction(octocrab::params::Direction::Descending)
                .per_page(100)
                .page(page)
                .send()
                .await?;

            if response.items.is_empty() {
                break;
            }

            let mut found_old = false;
            for pr in &response.items {
                let created = pr.created_at.unwrap_or(since);
                if created < since {
                    found_old = true;
                    continue;
                }

                let author = pr
                    .user
                    .as_ref()
                    .map(|u| u.login.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let time_to_merge = pr.merged_at.map(|merged| {
                    let duration = merged - created;
                    duration.num_minutes() as f64 / 60.0
                });

                prs.push(PullRequestSummary {
                    number: pr.number,
                    title: pr.title.clone().unwrap_or_default(),
                    author,
                    body: pr.body.clone(),
                    state: pr
                        .state
                        .as_ref()
                        .map(|s| format!("{:?}", s).to_lowercase())
                        .unwrap_or_else(|| "unknown".to_string()),
                    created_at: created,
                    merged_at: pr.merged_at,
                    closed_at: pr.closed_at,
                    additions: None,
                    deletions: None,
                    changed_files: None,
                    review_comments: 0,
                    time_to_merge_hours: time_to_merge,
                });
            }

            if found_old || response.next.is_none() {
                break;
            }
            page += 1;
        }

        Ok(prs)
    }
}
