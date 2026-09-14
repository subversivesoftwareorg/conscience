use conscience::github::client::GitHubClient;

#[test]
fn parse_pr_url_full() {
    let (owner, repo, number) = GitHubClient::parse_pr_url(
        "https://github.com/subversivesoftwareorg/conscience/pull/42",
    )
    .unwrap();
    assert_eq!(owner, "subversivesoftwareorg");
    assert_eq!(repo, "conscience");
    assert_eq!(number, 42);
}

#[test]
fn parse_pr_url_trailing_slash() {
    let (_, _, n) = GitHubClient::parse_pr_url(
        "https://github.com/org/repo/pull/7/",
    )
    .unwrap();
    assert_eq!(n, 7);
}

#[test]
fn parse_pr_url_short_form() {
    let (owner, repo, number) = GitHubClient::parse_pr_url("org/repo#123").unwrap();
    assert_eq!(owner, "org");
    assert_eq!(repo, "repo");
    assert_eq!(number, 123);
}

#[test]
fn parse_pr_url_invalid() {
    assert!(GitHubClient::parse_pr_url("not-a-url").is_err());
    assert!(GitHubClient::parse_pr_url("https://github.com/org/repo").is_err());
    assert!(GitHubClient::parse_pr_url("org/repo").is_err());
}
