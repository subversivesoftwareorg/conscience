use crate::error::Result;
use crate::github::auth;
use crate::github::client::GitHubClient;
use crate::github::models::RepoSummary;
use chrono::{Duration, Utc};

pub async fn ingest_github(repo: &str, days: u32) -> Result<RepoSummary> {
    let token = auth::resolve_token()?;
    let octocrab = auth::build_client(&token)?;
    let client = GitHubClient::new(octocrab);

    let (owner, repo_name) = GitHubClient::parse_repo(repo)?;
    let until = Utc::now();
    let since = until - Duration::days(days as i64);

    eprintln!(
        "Fetching data for {}/{} from {} to {}",
        owner,
        repo_name,
        since.format("%Y-%m-%d"),
        until.format("%Y-%m-%d")
    );

    let summary = client
        .fetch_summary(&owner, &repo_name, since, until)
        .await?;

    eprintln!(
        "Found {} commits and {} pull requests",
        summary.commits.len(),
        summary.pull_requests.len()
    );

    Ok(summary)
}
