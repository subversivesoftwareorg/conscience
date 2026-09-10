use crate::ai_tools::models::{AiSession, AiUsageSummary};
use crate::ethics::models::{Principle, Severity, Signal};
use crate::github::models::RepoSummary;
use chrono::Duration;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A commit that correlates with an AI session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCorrelatedCommit {
    pub sha: String,
    pub message: String,
    pub session_id: String,
    pub session_writes: u64,
    pub session_output_tokens: u64,
}

/// Per-contributor authorship analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorAuthorship {
    pub author: String,
    pub total_commits: usize,
    pub ai_correlated_commits: usize,
    pub ai_correlation_pct: f64,
    pub total_ai_writes: u64,
    pub correlated_commits: Vec<AiCorrelatedCommit>,
}

/// Full authorship analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorshipAnalysis {
    pub contributors: Vec<ContributorAuthorship>,
    pub total_commits: usize,
    pub total_ai_correlated: usize,
    pub overall_ai_correlation_pct: f64,
    pub signals: Vec<Signal>,
}

/// Cross-reference GitHub commits with AI sessions to detect
/// how much code is being written by AI vs. humans.
///
/// The correlation logic: a commit is "AI-correlated" if it was
/// made within a window of an AI session that had Write/Edit operations.
/// This is a heuristic — it detects temporal proximity, not causation.
pub fn analyze_authorship(
    github: &RepoSummary,
    ai: &AiUsageSummary,
) -> AuthorshipAnalysis {
    let window = Duration::minutes(30);

    let write_sessions: Vec<&AiSession> = ai
        .sessions
        .iter()
        .filter(|s| {
            let write_count: u64 = s
                .tools_used
                .get("Write")
                .cloned()
                .unwrap_or(0)
                + s.tools_used.get("Edit").cloned().unwrap_or(0);
            write_count > 0 && s.started_at.is_some() && s.ended_at.is_some()
        })
        .collect();

    let mut author_map: HashMap<String, ContributorAuthorship> = HashMap::new();
    let mut total_correlated = 0usize;

    for commit in &github.commits {
        let entry = author_map
            .entry(commit.author.clone())
            .or_insert_with(|| ContributorAuthorship {
                author: commit.author.clone(),
                total_commits: 0,
                ai_correlated_commits: 0,
                ai_correlation_pct: 0.0,
                total_ai_writes: 0,
                correlated_commits: Vec::new(),
            });

        entry.total_commits += 1;

        for session in &write_sessions {
            let session_start = session.started_at.unwrap();
            let session_end = session.ended_at.unwrap();

            let commit_in_window = commit.date >= session_start - window
                && commit.date <= session_end + window;

            if commit_in_window {
                let write_count = session
                    .tools_used
                    .get("Write")
                    .cloned()
                    .unwrap_or(0)
                    + session.tools_used.get("Edit").cloned().unwrap_or(0);

                entry.ai_correlated_commits += 1;
                entry.total_ai_writes += write_count;
                total_correlated += 1;

                entry.correlated_commits.push(AiCorrelatedCommit {
                    sha: commit.sha[..8].to_string(),
                    message: commit.message.lines().next().unwrap_or("").to_string(),
                    session_id: session.session_id[..8].to_string(),
                    session_writes: write_count,
                    session_output_tokens: session.tokens.output,
                });

                break; // one session match per commit is enough
            }
        }
    }

    // Compute percentages
    for contributor in author_map.values_mut() {
        contributor.ai_correlation_pct = if contributor.total_commits > 0 {
            contributor.ai_correlated_commits as f64 / contributor.total_commits as f64 * 100.0
        } else {
            0.0
        };
    }

    let mut contributors: Vec<ContributorAuthorship> = author_map.into_values().collect();
    contributors.sort_by(|a, b| {
        b.ai_correlation_pct
            .partial_cmp(&a.ai_correlation_pct)
            .unwrap()
    });

    let total_commits = github.commits.len();
    let overall_pct = if total_commits > 0 {
        total_correlated as f64 / total_commits as f64 * 100.0
    } else {
        0.0
    };

    let signals = generate_authorship_signals(&contributors, overall_pct);

    AuthorshipAnalysis {
        contributors,
        total_commits,
        total_ai_correlated: total_correlated,
        overall_ai_correlation_pct: overall_pct,
        signals,
    }
}

fn generate_authorship_signals(
    contributors: &[ContributorAuthorship],
    overall_pct: f64,
) -> Vec<Signal> {
    let mut signals = Vec::new();

    if overall_pct > 80.0 {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Warning,
            title: "Very high AI authorship correlation".to_string(),
            detail: format!(
                "{:.0}% of commits were made during or shortly after AI coding sessions. \
                Consider whether humans are writing code or primarily operating AI tools.",
                overall_pct
            ),
            evidence: "Correlation window: 30 minutes before/after AI sessions with Write/Edit operations".to_string(),
        });
    } else if overall_pct > 50.0 {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Info,
            title: "Majority of commits AI-correlated".to_string(),
            detail: format!(
                "{:.0}% of commits correlate with AI sessions. \
                This is common for AI-assisted development — the question is whether \
                humans are directing meaningfully or accepting output uncritically.",
                overall_pct
            ),
            evidence: "Correlation window: 30 minutes before/after AI sessions".to_string(),
        });
    }

    for contributor in contributors {
        if contributor.ai_correlation_pct > 90.0 && contributor.total_commits > 3 {
            signals.push(Signal {
                principle: Principle::DeveloperGrowth,
                severity: Severity::Concern,
                title: format!("{}: near-total AI authorship", contributor.author),
                detail: format!(
                    "{:.0}% of {}'s {} commits correlate with AI sessions ({} AI write operations). \
                    Is this person growing as a developer, or becoming a prompt operator?",
                    contributor.ai_correlation_pct,
                    contributor.author,
                    contributor.total_commits,
                    contributor.total_ai_writes
                ),
                evidence: format!(
                    "{} of {} commits AI-correlated",
                    contributor.ai_correlated_commits, contributor.total_commits
                ),
            });
        }
    }

    signals
}

pub fn print_authorship_analysis(analysis: &AuthorshipAnalysis) {
    println!();
    println!("  Authorship Analysis");
    println!(
        "  {} commits, {} AI-correlated ({:.0}%)",
        analysis.total_commits, analysis.total_ai_correlated, analysis.overall_ai_correlation_pct
    );
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Contributor").fg(Color::Cyan),
            Cell::new("Commits").fg(Color::Cyan),
            Cell::new("AI-Correlated").fg(Color::Cyan),
            Cell::new("AI %").fg(Color::Cyan),
            Cell::new("AI Writes").fg(Color::Cyan),
        ]);

    for c in &analysis.contributors {
        let pct_display = format!("{:.0}%", c.ai_correlation_pct);
        table.add_row(vec![
            c.author.clone(),
            c.total_commits.to_string(),
            c.ai_correlated_commits.to_string(),
            pct_display,
            c.total_ai_writes.to_string(),
        ]);
    }

    for line in table.to_string().lines() {
        println!("  {}", line);
    }

    // Show correlated commits for contributors with high AI correlation
    for c in &analysis.contributors {
        if c.ai_correlation_pct > 50.0 && !c.correlated_commits.is_empty() {
            println!();
            println!(
                "  {} \u{2014} AI-correlated commits:",
                c.author
            );

            let mut detail_table = Table::new();
            detail_table
                .load_preset(UTF8_FULL)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_header(vec![
                    Cell::new("Commit").fg(Color::DarkGrey),
                    Cell::new("Message").fg(Color::DarkGrey),
                    Cell::new("Session").fg(Color::DarkGrey),
                    Cell::new("Writes").fg(Color::DarkGrey),
                    Cell::new("Tokens").fg(Color::DarkGrey),
                ]);

            for cc in c.correlated_commits.iter().take(10) {
                let msg = if cc.message.chars().count() > 50 {
                    format!("{}...", cc.message.chars().take(47).collect::<String>())
                } else {
                    cc.message.clone()
                };
                let tokens = if cc.session_output_tokens >= 1_000 {
                    format!("{:.1}K", cc.session_output_tokens as f64 / 1_000.0)
                } else {
                    cc.session_output_tokens.to_string()
                };
                detail_table.add_row(vec![
                    cc.sha.clone(),
                    msg,
                    cc.session_id.clone(),
                    cc.session_writes.to_string(),
                    tokens,
                ]);
            }

            if c.correlated_commits.len() > 10 {
                println!(
                    "  (showing 10 of {})",
                    c.correlated_commits.len()
                );
            }

            for line in detail_table.to_string().lines() {
                println!("  {}", line);
            }
        }
    }

    if !analysis.signals.is_empty() {
        println!();
        println!("  Authorship Signals");
        println!();
        for signal in &analysis.signals {
            let color = match signal.severity {
                Severity::Warning => "\x1b[31m",
                Severity::Concern => "\x1b[33m",
                Severity::Info => "\x1b[36m",
                Severity::Healthy => "\x1b[32m",
            };
            let reset = "\x1b[0m";
            println!(
                "  {}{:>7}{} {}",
                color,
                signal.severity.to_string(),
                reset,
                signal.title,
            );
            println!("          {}", signal.detail);
            println!();
        }
    }
}
