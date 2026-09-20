//! Detect the GitHub repository a project directory is a checkout of.
//!
//! The current directory is the project, so its `origin` remote is the
//! natural default for `--repo`. Only GitHub remotes are recognised; any
//! other host means "no GitHub repo", not an error.

use std::path::Path;
use std::process::Command;

/// `owner/repo` from a remote URL, if it points at github.com.
///
/// Accepts `git@github.com:owner/repo.git`, `ssh://git@github.com/owner/repo`,
/// `https://github.com/owner/repo(.git)`, and `github.com/owner/repo`.
pub fn parse_github_remote(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = if let Some(r) = url.strip_prefix("git@github.com:") {
        r
    } else if let Some(r) = url.strip_prefix("ssh://git@github.com/") {
        r
    } else if let Some(r) = url.strip_prefix("https://github.com/") {
        r
    } else if let Some(r) = url.strip_prefix("http://github.com/") {
        r
    } else if let Some(r) = url.strip_prefix("github.com/") {
        r
    } else {
        return None;
    };
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = rest.splitn(3, '/');
    let owner = parts.next().filter(|s| !s.is_empty())?;
    let repo = parts.next().filter(|s| !s.is_empty())?;
    if parts.next().is_some() {
        return None; // deeper paths (tree/, pull/) are not a repository
    }
    Some(format!("{}/{}", owner, repo))
}

/// The `origin` remote of the git repository containing `root`, as
/// `owner/repo`, when it is on GitHub. `None` if `root` is not inside a
/// git repository, has no `origin`, git is unavailable, or the remote is
/// not on GitHub.
pub fn detect_origin_repo(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_github_remote(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ssh_https_and_bare_forms() {
        for url in [
            "git@github.com:acme/widget.git",
            "git@github.com:acme/widget",
            "ssh://git@github.com/acme/widget.git",
            "https://github.com/acme/widget",
            "https://github.com/acme/widget.git",
            "https://github.com/acme/widget/",
            "github.com/acme/widget",
            "  https://github.com/acme/widget\n",
        ] {
            assert_eq!(
                parse_github_remote(url).as_deref(),
                Some("acme/widget"),
                "{}",
                url
            );
        }
    }

    #[test]
    fn rejects_other_hosts_and_non_repo_paths() {
        for url in [
            "git@gitlab.com:acme/widget.git",
            "https://bitbucket.org/acme/widget",
            "https://github.com/acme",
            "https://github.com/acme/widget/pull/3",
            "https://github.com//widget",
            "",
            "not a url",
        ] {
            assert_eq!(parse_github_remote(url), None, "{}", url);
        }
    }
}
