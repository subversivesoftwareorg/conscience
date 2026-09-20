//! Post a PR-scoped analysis as a pull request comment.
//!
//! The comment is rendered from the sanitized export, never the raw
//! snapshot: a PR comment is visible to everyone with repository access, so
//! it must not carry file paths, shell commands, session ids, or names that
//! appeared in signal evidence. A hidden marker lets a re-run edit its own
//! earlier comment instead of adding another.

use crate::ethics::models::Severity;
use crate::export::{ExportSignal, SnapshotExport};
use crate::snapshot::SourceStatus;
use std::fmt::Write as _;

/// Hidden in every comment conscience posts, so it can find it again.
pub const MARKER: &str = "<!-- conscience:pr-comment -->";

/// The Markdown body for a pull request comment.
pub fn render_pr_comment(export: &SnapshotExport, pr_number: u64) -> String {
    let a = &export.analysis;
    let mut md = String::new();

    let _ = writeln!(md, "{}", MARKER);
    let _ = writeln!(md, "## Conscience \u{2014} PR #{}", pr_number);
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "Analyzed {} to {} (the PR's lifetime). Coverage: {}.",
        export.interval.start.format("%Y-%m-%d"),
        export.interval.end.format("%Y-%m-%d"),
        export
            .coverage
            .sources
            .iter()
            .map(|s| format!("{} {}", s.source, s.status))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(md);

    // Manifest signals are about the project's conscience.yaml, not the
    // change under review; they belong in `examine`, not on a PR.
    let relevant: Vec<&ExportSignal> = a.signals.iter().filter(|s| about_the_pr(s)).collect();
    let mut attention: Vec<&ExportSignal> = relevant
        .iter()
        .copied()
        .filter(|s| s.severity >= Severity::Concern)
        .collect();
    attention.sort_by_key(|s| std::cmp::Reverse(s.severity));
    let quiet = relevant.len() - attention.len();

    if attention.is_empty() {
        let _ = writeln!(
            md,
            "**No concerns or warnings detected in the available data.** ({} signal(s) at info or healthy level.)",
            quiet
        );
    } else {
        let _ = writeln!(md, "**{} signal(s) deserve attention:**", attention.len());
        let _ = writeln!(md);
        for s in &attention {
            let icon = match s.severity {
                Severity::Warning => "\u{1F534}",
                _ => "\u{1F7E1}",
            };
            let _ = writeln!(md, "{} **{}** [{}]", icon, s.title, s.principle.name());
            let _ = writeln!(md, "> {}", s.detail);
            if !s.evidence.is_empty() {
                let _ = writeln!(md, "> _{}_", s.evidence);
            }
            let _ = writeln!(md);
        }
        if quiet > 0 {
            let _ = writeln!(md, "{} more signal(s) at info or healthy level.", quiet);
            let _ = writeln!(md);
        }
    }

    // One question, for the principle with the most serious signal.
    let focus = attention.first().map(|s| s.principle);
    let question = focus
        .and_then(|p| a.reflections.iter().find(|r| r.principle == p))
        .or_else(|| a.reflections.first());
    if let Some(q) = question {
        let _ = writeln!(md, "**Worth discussing** ({}):", q.principle.name());
        let _ = writeln!(md);
        let _ = writeln!(md, "> {}", q.question);
        if !q.data_context.is_empty() {
            let _ = writeln!(md, ">");
            let _ = writeln!(md, "> _{}_", q.data_context);
        }
        let _ = writeln!(md);
    }

    let _ = writeln!(md, "<details><summary>Scorecard</summary>");
    let _ = writeln!(md);
    let _ = writeln!(md, "| Principle | Signals | Status |");
    let _ = writeln!(md, "|---|---|---|");
    for d in &a.scorecard {
        let auto: Vec<&ExportSignal> = d.auto_signals.iter().filter(|s| about_the_pr(s)).collect();
        let worst = auto.iter().map(|s| s.severity).max();
        let status = match (worst, d.needs_human_input) {
            (None, true) => "Needs human input".to_string(),
            (None, false) => "No data".to_string(),
            (Some(w), true) => format!("{} (+ needs human input)", w),
            (Some(w), false) => w.to_string(),
        };
        let _ = writeln!(
            md,
            "| {} | {} | {} |",
            d.principle.name(),
            auto.len(),
            status
        );
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "</details>");
    let _ = writeln!(md);

    // CI runners never have AI session logs. Say so, and say how to add
    // them: a local run edits this same comment rather than adding another.
    let ai_collected = export
        .coverage
        .sources
        .iter()
        .any(|s| s.source == "claude_code" && s.status == SourceStatus::Collected);
    if !ai_collected {
        let pr_ref = match &export.project.github_repo {
            Some(repo) => format!("{}#{}", repo, pr_number),
            None => format!("#{}", pr_number),
        };
        let _ = writeln!(
            md,
            "_No AI session data was available where this ran. To add it, run `conscience examine --pr {} --comment` on the machine where the work happened; it updates this comment in place._",
            pr_ref
        );
        let _ = writeln!(md);
    }
    let _ = writeln!(
        md,
        "<sub>Signals are observations for human judgment, not verdicts. conscience {} \u{00b7} snapshot `{}` \u{00b7} evidence with paths, commands, or names stays on the machine that ran the analysis.</sub>",
        export.analyzer.version, export.snapshot_id
    );

    md
}

/// Signals worth putting on a pull request: everything except the
/// `manifest_*` family, which describes the project's configuration.
pub fn about_the_pr(s: &ExportSignal) -> bool {
    !s.id.starts_with("manifest_")
}

/// Given existing comments as `(id, body)`, the one conscience posted
/// earlier, if any. Pure so it can be tested without a network.
pub fn find_existing(comments: &[(u64, String)]) -> Option<u64> {
    comments
        .iter()
        .find(|(_, body)| body.contains(MARKER))
        .map(|(id, _)| *id)
}

/// What happened when posting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentOutcome {
    Created(String),
    Updated(String),
}

impl crate::github::client::GitHubClient {
    /// Post `body` on a pull request, editing conscience's earlier comment
    /// on that PR if there is one.
    pub async fn upsert_pr_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        body: &str,
    ) -> crate::error::Result<CommentOutcome> {
        let issues = self.octocrab().issues(owner, repo);

        let page = issues.list_comments(number).per_page(100).send().await?;
        let existing: Vec<(u64, String)> = page
            .items
            .iter()
            .map(|c| (c.id.into_inner(), c.body.clone().unwrap_or_default()))
            .collect();

        match find_existing(&existing) {
            Some(id) => {
                let c = issues
                    .update_comment(octocrab::models::CommentId(id), body)
                    .await?;
                Ok(CommentOutcome::Updated(c.html_url.to_string()))
            }
            None => {
                let c = issues.create_comment(number, body).await?;
                Ok(CommentOutcome::Created(c.html_url.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_only_the_marked_comment() {
        let comments = vec![
            (1, "LGTM".to_string()),
            (2, format!("{}\n## Conscience", MARKER)),
            (3, "another marked one? no".to_string()),
        ];
        assert_eq!(find_existing(&comments), Some(2));
        assert_eq!(find_existing(&comments[..1]), None);
        assert_eq!(find_existing(&[]), None);
    }
}
