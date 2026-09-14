use crate::error::Result;
use crate::github::auth;
use crate::github::client::GitHubClient;
use crate::github::models::RepoSummary;
use chrono::{Duration, Utc};

pub async fn ingest_pr(pr_url: &str) -> Result<RepoSummary> {
    let (owner, repo_name, number) = GitHubClient::parse_pr_url(pr_url)?;

    let token = auth::resolve_token()?;
    let octocrab = auth::build_client(&token)?;
    let client = GitHubClient::new(octocrab);

    eprintln!("Fetching PR #{} from {}/{}...", number, owner, repo_name);

    let pr = client.fetch_single_pr(&owner, &repo_name, number).await?;
    let commits = client.fetch_pr_commits(&owner, &repo_name, number).await?;

    eprintln!(
        "PR #{}: \"{}\" by {} ({} commits, +{}/−{})",
        pr.number,
        pr.title,
        pr.author,
        commits.len(),
        pr.additions.unwrap_or(0),
        pr.deletions.unwrap_or(0),
    );

    Ok(RepoSummary {
        owner,
        repo: repo_name,
        period_start: pr.created_at,
        period_end: pr.merged_at.or(pr.closed_at).unwrap_or_else(chrono::Utc::now),
        commits,
        pull_requests: vec![pr],
    })
}

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
