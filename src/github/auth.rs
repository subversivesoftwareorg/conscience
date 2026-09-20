use crate::config::Config;
use crate::error::{ConscienceError, Result};
use std::process::Command;

/// Is a token obtainable from any source? Quiet: prints nothing, so it can
/// gate optional GitHub collection without noise on every run.
pub fn token_available() -> bool {
    if std::env::var("CONSCIENCE_GITHUB_TOKEN").is_ok_and(|t| !t.is_empty()) {
        return true;
    }
    if Config::load().github.token.is_some_and(|t| !t.is_empty()) {
        return true;
    }
    Command::new("gh")
        .args(["auth", "token"])
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.is_empty())
}

/// Resolve a GitHub token using a layered strategy:
/// 1. CONSCIENCE_GITHUB_TOKEN env var
/// 2. Config file (~/.conscience/config.toml)
/// 3. `gh auth token` (GitHub CLI)
pub fn resolve_token() -> Result<String> {
    if let Ok(token) = std::env::var("CONSCIENCE_GITHUB_TOKEN") {
        if !token.is_empty() {
            eprintln!("Using GitHub token from CONSCIENCE_GITHUB_TOKEN");
            return Ok(token);
        }
    }

    let config = Config::load();
    if let Some(token) = config.github.token {
        if !token.is_empty() {
            eprintln!("Using GitHub token from ~/.conscience/config.toml");
            return Ok(token);
        }
    }

    match Command::new("gh").args(["auth", "token"]).output() {
        Ok(output) if output.status.success() => {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                eprintln!("Using GitHub token from `gh auth token`");
                return Ok(token);
            }
            Err(ConscienceError::NoToken)
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ConscienceError::AuthError(stderr.trim().to_string()))
        }
        Err(_) => Err(ConscienceError::NoToken),
    }
}

pub fn build_client(token: &str) -> Result<octocrab::Octocrab> {
    octocrab::Octocrab::builder()
        .personal_token(token.to_string())
        .build()
        .map_err(|e| ConscienceError::AuthError(e.to_string()))
}
