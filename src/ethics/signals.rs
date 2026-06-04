use crate::ai_tools::models::AiUsageSummary;
use crate::ethics::manifest::{Manifest, Thresholds};
use crate::ethics::models::*;
use crate::github::models::RepoSummary;
use std::collections::HashMap;

/// Detect ethical signals from GitHub data.
pub fn detect_github_signals(
    summary: &RepoSummary,
    thresholds: Option<&Thresholds>,
) -> Vec<Signal> {
    let defaults = Thresholds::default();
    let t = thresholds.unwrap_or(&defaults);
    let mut signals = Vec::new();

    if !t.solo_project {
        detect_contribution_concentration(summary, t, &mut signals);
    }
    detect_review_patterns(summary, &mut signals);
    detect_velocity_signals(summary, &mut signals);
    detect_prompt_injection_risk(summary, &mut signals);

    signals
}

/// Detect ethical signals from AI usage data.
pub fn detect_ai_signals(
    summary: &AiUsageSummary,
    thresholds: Option<&Thresholds>,
) -> Vec<Signal> {
    let defaults = Thresholds::default();
    let t = thresholds.unwrap_or(&defaults);
    let mut signals = Vec::new();

    detect_ai_dependency(summary, t, &mut signals);
    detect_token_consumption(summary, &mut signals);
    detect_tool_patterns(summary, &mut signals);
    detect_tokenmaxxing(summary, t, &mut signals);
    detect_sensitive_file_access(summary, &mut signals);
    detect_suspicious_bash(summary, &mut signals);
    detect_agent_action_concerns(summary, &mut signals);

    signals
}

/// Detect signals from the conscience.yaml manifest itself.
pub fn detect_manifest_signals(manifest: &Manifest) -> Vec<Signal> {
    let mut signals = Vec::new();

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if manifest.monthly_review.is_stale(&today) {
        signals.push(Signal {
            principle: Principle::Transparency,
            severity: Severity::Concern,
            title: "Monthly review is stale".to_string(),
            detail: format!(
                "Last updated: {}. Conscience works best when the monthly review \
                reflects current reality. Run `conscience examine` and update conscience.yaml.",
                if manifest.monthly_review.last_updated.is_empty() {
                    "never"
                } else {
                    &manifest.monthly_review.last_updated
                }
            ),
            evidence: "conscience.yaml monthly_review.last_updated".to_string(),
        });
    }

    if manifest.project.beneficiaries.is_empty() {
        signals.push(Signal {
            principle: Principle::EquityOfBenefit,
            severity: Severity::Info,
            title: "No beneficiaries defined".to_string(),
            detail: "conscience.yaml doesn't name who benefits from this project. \
                Articulating beneficiaries makes ethical analysis possible."
                .to_string(),
            evidence: "conscience.yaml project.beneficiaries is empty".to_string(),
        });
    }

    if !manifest.monthly_review.value_delivered.is_empty() {
        signals.push(Signal {
            principle: Principle::Transparency,
            severity: Severity::Healthy,
            title: "Value delivery documented".to_string(),
            detail: "Team has articulated what value was delivered this period.".to_string(),
            evidence: format!(
                "\"{}\"",
                truncate(&manifest.monthly_review.value_delivered, 120)
            ),
        });
    }

    if !manifest.monthly_review.ai_sentiment.is_empty() {
        let sentiment = &manifest.monthly_review.ai_sentiment;
        let (severity, detail) = match sentiment.as_str() {
            "flourishing" => (
                Severity::Healthy,
                "Team reports AI is helping them flourish.",
            ),
            "mostly_positive" => (Severity::Healthy, "Team sentiment toward AI is mostly positive."),
            "mixed" => (Severity::Info, "Team has mixed feelings about AI's impact."),
            "concerning" => (
                Severity::Concern,
                "Team reports concerns about AI's impact on their work.",
            ),
            "harmful" => (
                Severity::Warning,
                "Team reports AI is actively harmful to their work.",
            ),
            _ => (Severity::Info, "AI sentiment recorded."),
        };
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity,
            title: "AI sentiment self-assessment".to_string(),
            detail: detail.to_string(),
            evidence: format!("conscience.yaml ai_sentiment: \"{}\"", sentiment),
        });
    }

    if manifest.team.roles.junior > 0 && manifest.team.learning_goals.is_empty() {
        signals.push(Signal {
            principle: Principle::DeveloperGrowth,
            severity: Severity::Info,
            title: "Junior developers without learning goals".to_string(),
            detail: format!(
                "Team has {} junior developer(s) but no learning goals defined. \
                Consider what growth looks like for them in an AI-assisted environment.",
                manifest.team.roles.junior
            ),
            evidence: "conscience.yaml team.roles.junior > 0, learning_goals empty".to_string(),
        });
    }

    signals
}

// --- GitHub signal detectors ---

fn detect_contribution_concentration(
    summary: &RepoSummary,
    thresholds: &Thresholds,
    signals: &mut Vec<Signal>,
) {
    if summary.commits.is_empty() {
        return;
    }

    let mut author_commits: HashMap<&str, usize> = HashMap::new();
    for commit in &summary.commits {
        *author_commits.entry(&commit.author).or_insert(0) += 1;
    }

    let total = summary.commits.len();
    let num_authors = author_commits.len();

    if num_authors < 2 {
        if total > 5 {
            signals.push(Signal {
                principle: Principle::EquityOfBenefit,
                severity: Severity::Concern,
                title: "Single contributor".to_string(),
                detail: "All commits come from one person. Is the team collaborating, or is one person doing everything with AI?".to_string(),
                evidence: format!(
                    "{} commits, all from {}",
                    total,
                    author_commits.keys().next().unwrap_or(&"unknown")
                ),
            });
        }
        return;
    }

    let mut counts: Vec<usize> = author_commits.values().cloned().collect();
    counts.sort_unstable_by(|a, b| b.cmp(a));

    let top_two: usize = counts.iter().take(2).sum();
    let concentration = top_two as f64 / total as f64;

    if concentration > thresholds.contribution_concentration_warn && num_authors > 3 {
        signals.push(Signal {
            principle: Principle::EquityOfBenefit,
            severity: Severity::Warning,
            title: "High contribution concentration".to_string(),
            detail: format!(
                "Top 2 of {} contributors account for {:.0}% of commits. \
                AI may be amplifying existing imbalances rather than distributing capability.",
                num_authors,
                concentration * 100.0
            ),
            evidence: format!("{} commits across {} authors", total, num_authors),
        });
    } else if concentration > thresholds.contribution_concentration_concern && num_authors > 3 {
        signals.push(Signal {
            principle: Principle::EquityOfBenefit,
            severity: Severity::Info,
            title: "Moderate contribution concentration".to_string(),
            detail: format!(
                "Top 2 of {} contributors account for {:.0}% of commits.",
                num_authors,
                concentration * 100.0
            ),
            evidence: format!("{} commits across {} authors", total, num_authors),
        });
    } else {
        signals.push(Signal {
            principle: Principle::EquityOfBenefit,
            severity: Severity::Healthy,
            title: "Distributed contributions".to_string(),
            detail: format!(
                "Commits are spread across {} contributors.",
                num_authors
            ),
            evidence: format!(
                "{} commits, top 2 account for {:.0}%",
                total,
                concentration * 100.0
            ),
        });
    }
}

fn detect_review_patterns(summary: &RepoSummary, signals: &mut Vec<Signal>) {
    if summary.pull_requests.is_empty() {
        return;
    }

    let merged: Vec<_> = summary
        .pull_requests
        .iter()
        .filter(|pr| pr.merged_at.is_some())
        .collect();

    if merged.is_empty() {
        return;
    }

    let no_review: Vec<_> = merged
        .iter()
        .filter(|pr| pr.review_comments == 0)
        .collect();

    let no_review_pct = no_review.len() as f64 / merged.len() as f64;

    if no_review_pct > 0.5 && merged.len() > 3 {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Concern,
            title: "Low review engagement".to_string(),
            detail: format!(
                "{:.0}% of merged PRs had zero review comments. \
                Are humans reviewing AI-generated code, or rubber-stamping it?",
                no_review_pct * 100.0
            ),
            evidence: format!(
                "{} of {} merged PRs with no review comments",
                no_review.len(),
                merged.len()
            ),
        });
    }

    let fast_merges: Vec<_> = merged
        .iter()
        .filter(|pr| pr.time_to_merge_hours.is_some_and(|h| h < 0.25))
        .collect();

    if fast_merges.len() > 3 {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Info,
            title: "Very fast merge times".to_string(),
            detail: format!(
                "{} PRs merged in under 15 minutes. \
                Fast can be good, but consider whether sufficient review occurred.",
                fast_merges.len()
            ),
            evidence: format!(
                "{} of {} merged PRs under 15min",
                fast_merges.len(),
                merged.len()
            ),
        });
    }
}

fn detect_velocity_signals(summary: &RepoSummary, signals: &mut Vec<Signal>) {
    let merged_count = summary
        .pull_requests
        .iter()
        .filter(|pr| pr.merged_at.is_some())
        .count();

    let days = (summary.period_end - summary.period_start).num_days().max(1);
    let prs_per_week = merged_count as f64 / (days as f64 / 7.0);

    if prs_per_week > 0.0 {
        signals.push(Signal {
            principle: Principle::Transparency,
            severity: Severity::Info,
            title: "Velocity baseline".to_string(),
            detail: format!(
                "{:.1} PRs merged per week over {} days. \
                Track this over time to see if AI adoption changes the pace.",
                prs_per_week, days
            ),
            evidence: format!("{} PRs merged in {} days", merged_count, days),
        });
    }
}

// --- AI usage signal detectors ---

fn detect_ai_dependency(
    summary: &AiUsageSummary,
    thresholds: &Thresholds,
    signals: &mut Vec<Signal>,
) {
    if summary.session_count == 0 {
        return;
    }

    let ratio = if summary.total_turns.human > 0 {
        summary.total_turns.assistant as f64 / summary.total_turns.human as f64
    } else {
        0.0
    };

    if ratio > thresholds.ai_dependency_concern {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Concern,
            title: "High AI:Human turn ratio".to_string(),
            detail: format!(
                "AI produces {:.1}x more turns than humans. \
                This may indicate the AI is driving the work rather than the human directing it.",
                ratio
            ),
            evidence: format!(
                "{} assistant turns vs {} human turns",
                summary.total_turns.assistant, summary.total_turns.human
            ),
        });
    } else if ratio > thresholds.ai_dependency_info {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Info,
            title: "Moderate AI:Human turn ratio".to_string(),
            detail: format!("AI produces {:.1}x more turns than humans.", ratio),
            evidence: format!(
                "{} assistant turns vs {} human turns",
                summary.total_turns.assistant, summary.total_turns.human
            ),
        });
    } else if summary.total_turns.total > 0 {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Healthy,
            title: "Balanced AI:Human interaction".to_string(),
            detail: format!(
                "AI:Human ratio of {:.1}:1 suggests humans are directing the work.",
                ratio
            ),
            evidence: format!(
                "{} assistant turns vs {} human turns",
                summary.total_turns.assistant, summary.total_turns.human
            ),
        });
    }

    let write_ops = summary.tools_used.get("Write").cloned().unwrap_or(0);
    let edit_ops = summary.tools_used.get("Edit").cloned().unwrap_or(0);
    let read_ops = summary.tools_used.get("Read").cloned().unwrap_or(0);
    let total_file_ops = write_ops + edit_ops + read_ops;

    if total_file_ops > 0 {
        let write_ratio = write_ops as f64 / total_file_ops as f64;
        if write_ratio > 0.6 && write_ops > 10 {
            signals.push(Signal {
                principle: Principle::CodeProvenance,
                severity: Severity::Info,
                title: "High ratio of new file creation".to_string(),
                detail: format!(
                    "{:.0}% of file operations are Write (new files) vs Edit (modifying existing). \
                    AI may be generating large amounts of new code rather than working within existing patterns.",
                    write_ratio * 100.0
                ),
                evidence: format!("{} writes, {} edits, {} reads", write_ops, edit_ops, read_ops),
            });
        }
    }
}

fn detect_token_consumption(summary: &AiUsageSummary, signals: &mut Vec<Signal>) {
    let total = summary.total_tokens.total();
    if total == 0 {
        return;
    }

    let output = summary.total_tokens.output;

    if output > 1_000_000 {
        signals.push(Signal {
            principle: Principle::EnvironmentalCost,
            severity: Severity::Info,
            title: "Significant token consumption".to_string(),
            detail: format!(
                "{:.1}M output tokens across {} sessions. \
                Consider whether the AI usage is proportionate to the value delivered.",
                output as f64 / 1_000_000.0,
                summary.session_count
            ),
            evidence: format!(
                "Total: {:.1}M tokens ({:.1}M output, {:.1}M cache)",
                total as f64 / 1_000_000.0,
                output as f64 / 1_000_000.0,
                (summary.total_tokens.cache_creation + summary.total_tokens.cache_read) as f64
                    / 1_000_000.0
            ),
        });
    }

    let cache_read = summary.total_tokens.cache_read;
    let cache_create = summary.total_tokens.cache_creation;
    if cache_create > 0 {
        let cache_efficiency = cache_read as f64 / (cache_read + cache_create) as f64;
        if cache_efficiency > 0.8 {
            signals.push(Signal {
                principle: Principle::EnvironmentalCost,
                severity: Severity::Healthy,
                title: "Good cache efficiency".to_string(),
                detail: format!(
                    "{:.0}% cache hit rate — reusing context rather than recomputing it.",
                    cache_efficiency * 100.0
                ),
                evidence: format!(
                    "{:.1}M cache reads vs {:.1}M cache creates",
                    cache_read as f64 / 1_000_000.0,
                    cache_create as f64 / 1_000_000.0
                ),
            });
        }
    }
}

fn detect_tool_patterns(summary: &AiUsageSummary, signals: &mut Vec<Signal>) {
    let bash_count = summary.tools_used.get("Bash").cloned().unwrap_or(0);
    let total_tools: u64 = summary.tools_used.values().sum();

    if total_tools > 0 && bash_count as f64 / total_tools as f64 > 0.5 {
        signals.push(Signal {
            principle: Principle::Transparency,
            severity: Severity::Info,
            title: "AI executing many shell commands".to_string(),
            detail: format!(
                "{} of {} tool invocations are Bash commands. \
                Shell commands can have side effects — is the team reviewing what AI executes?",
                bash_count, total_tools
            ),
            evidence: format!(
                "Bash: {}, Read: {}, Edit: {}, Write: {}",
                bash_count,
                summary.tools_used.get("Read").unwrap_or(&0),
                summary.tools_used.get("Edit").unwrap_or(&0),
                summary.tools_used.get("Write").unwrap_or(&0),
            ),
        });
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// --- Security signal detectors ---

const SENSITIVE_FILENAMES: &[&str] = &[
    ".env", ".env.local", ".env.production",
    "credentials", "credentials.json", "credentials.yaml",
    "id_rsa", "id_ed25519", "id_ecdsa",
    ".pem", ".key", ".p12", ".pfx", ".jks", ".keystore",
    "secret", "secrets.yaml", "secrets.json",
    "aws_access", ".aws",
    "service_account", "serviceAccountKey",
    "password", "passwords",
];

fn is_sensitive_path(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path).to_lowercase();
    SENSITIVE_FILENAMES.iter().any(|pat| {
        let pat = pat.to_lowercase();
        filename == pat || filename.ends_with(&pat) || filename.starts_with(pat.trim_start_matches('.'))
    })
}

fn detect_tokenmaxxing(
    summary: &AiUsageSummary,
    thresholds: &Thresholds,
    signals: &mut Vec<Signal>,
) {
    for session in &summary.sessions {
        let files_count = session.files_touched.len().max(1) as u64;
        let tokens_per_file = session.tokens.output / files_count;

        if tokens_per_file > thresholds.tokens_per_file_warn && session.tokens.output > 10_000 {
            signals.push(Signal {
                principle: Principle::Security,
                severity: Severity::Concern,
                title: "High token-to-file ratio".to_string(),
                detail: format!(
                    "Session {} used {}K output tokens but only touched {} file(s). \
                    Lots of tokens with little tangible output may indicate waste or misuse.",
                    &session.session_id[..session.session_id.len().min(8)],
                    session.tokens.output / 1_000,
                    session.files_touched.len()
                ),
                evidence: format!(
                    "{}K tokens / {} files = {}K tokens/file (threshold: {}K)",
                    session.tokens.output / 1_000,
                    session.files_touched.len(),
                    tokens_per_file / 1_000,
                    thresholds.tokens_per_file_warn / 1_000
                ),
            });
        }

        if let Some(tokens_per_turn) = session.tokens.output.checked_div(session.turns.assistant) {
            if tokens_per_turn > thresholds.tokens_per_turn_warn {
                signals.push(Signal {
                    principle: Principle::Security,
                    severity: Severity::Info,
                    title: "High tokens per turn".to_string(),
                    detail: format!(
                        "Session {} averaged {}K output tokens per turn. \
                        AI may be generating excessive content.",
                        &session.session_id[..session.session_id.len().min(8)],
                        tokens_per_turn / 1_000,
                    ),
                    evidence: format!(
                        "{}K output tokens / {} turns",
                        session.tokens.output / 1_000,
                        session.turns.assistant
                    ),
                });
            }
        }

        if let (Some(start), Some(end)) = (session.started_at, session.ended_at) {
            let duration_hours = (end - start).num_minutes() as f64 / 60.0;
            if duration_hours > thresholds.max_session_hours {
                signals.push(Signal {
                    principle: Principle::Security,
                    severity: Severity::Warning,
                    title: "Extremely long AI session".to_string(),
                    detail: format!(
                        "Session {} ran for {:.1} hours. \
                        This may indicate unattended AI automation.",
                        &session.session_id[..session.session_id.len().min(8)],
                        duration_hours
                    ),
                    evidence: format!(
                        "Started: {}, Ended: {}, Duration: {:.1}h (threshold: {:.0}h)",
                        start.format("%Y-%m-%d %H:%M"),
                        end.format("%Y-%m-%d %H:%M"),
                        duration_hours,
                        thresholds.max_session_hours
                    ),
                });
            }
        }
    }
}

fn detect_sensitive_file_access(summary: &AiUsageSummary, signals: &mut Vec<Signal>) {
    let mut sensitive_reads: Vec<String> = Vec::new();
    let mut sensitive_writes: Vec<String> = Vec::new();

    for session in &summary.sessions {
        for file in &session.files_touched {
            if is_sensitive_path(&file.path) {
                match file.action {
                    crate::ai_tools::models::FileAction::Read => {
                        sensitive_reads.push(file.path.clone());
                    }
                    crate::ai_tools::models::FileAction::Write
                    | crate::ai_tools::models::FileAction::Edit => {
                        sensitive_writes.push(file.path.clone());
                    }
                }
            }
        }
    }

    if !sensitive_writes.is_empty() {
        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Warning,
            title: "AI wrote to sensitive files".to_string(),
            detail: format!(
                "AI wrote or edited {} file(s) that may contain secrets or credentials. \
                Verify these changes don't introduce or expose sensitive data.",
                sensitive_writes.len()
            ),
            evidence: sensitive_writes.join(", "),
        });
    }

    if !sensitive_reads.is_empty() {
        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Concern,
            title: "AI read sensitive files".to_string(),
            detail: format!(
                "AI read {} file(s) that may contain secrets. \
                Sensitive content may have been included in AI context.",
                sensitive_reads.len()
            ),
            evidence: sensitive_reads.join(", "),
        });
    }
}

fn detect_suspicious_bash(summary: &AiUsageSummary, signals: &mut Vec<Signal>) {
    let mut network_exfil: Vec<String> = Vec::new();
    let mut encoding_ops: Vec<String> = Vec::new();
    let mut credential_access: Vec<String> = Vec::new();

    for session in &summary.sessions {
        for cmd in &session.bash_commands {
            let lower = cmd.to_lowercase();

            let is_network_exfil =
                ((lower.contains("curl") || lower.contains("wget")) && lower.contains("|"))
                || lower.contains("netcat") || lower.contains(" nc ") || lower.starts_with("nc ")
                || ((lower.contains("scp ") || lower.contains("rsync ")) && lower.contains("@"));

            if is_network_exfil {
                network_exfil.push(truncate(cmd, 80));
            }

            let is_encoding =
                (lower.contains("base64") && lower.contains("|"))
                || lower.contains("openssl enc");

            if is_encoding {
                encoding_ops.push(truncate(cmd, 80));
            }

            // Credential directory access
            if lower.contains("/.ssh/") || lower.contains("/.aws/") || lower.contains("/.gnupg/") {
                credential_access.push(truncate(cmd, 80));
            }
        }
    }

    if !network_exfil.is_empty() {
        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Warning,
            title: "Potential network exfiltration".to_string(),
            detail: format!(
                "{} bash command(s) involve piped network operations. \
                Review these commands to ensure data isn't being sent to external hosts.",
                network_exfil.len()
            ),
            evidence: network_exfil.join("\n"),
        });
    }

    if !encoding_ops.is_empty() {
        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Concern,
            title: "Encoding/obfuscation commands".to_string(),
            detail: format!(
                "{} bash command(s) involve encoding operations (base64, openssl). \
                These can be legitimate but are also used to obscure data exfiltration.",
                encoding_ops.len()
            ),
            evidence: encoding_ops.join("\n"),
        });
    }

    if !credential_access.is_empty() {
        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Concern,
            title: "AI accessed credential directories".to_string(),
            detail: format!(
                "{} bash command(s) accessed SSH, AWS, or GPG credential directories.",
                credential_access.len()
            ),
            evidence: credential_access.join("\n"),
        });
    }
}

fn detect_agent_action_concerns(summary: &AiUsageSummary, signals: &mut Vec<Signal>) {
    let actions = &summary.agent_actions_summary;

    if actions.total_actions == 0 {
        return;
    }

    // Messages/emails sent without approval
    if actions.actions_without_approval > 0 {
        let severity = if actions.actions_without_approval > 10 {
            Severity::Warning
        } else {
            Severity::Concern
        };
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity,
            title: "Agent actions without human approval".to_string(),
            detail: format!(
                "{} action(s) were taken without explicit human approval. \
                When AI sends messages or modifies calendars on your behalf, \
                recipients may not know they're interacting with an AI.",
                actions.actions_without_approval
            ),
            evidence: format!(
                "{} unapproved of {} total actions",
                actions.actions_without_approval, actions.total_actions
            ),
        });
    }

    if actions.emails_sent > 0 || actions.messages_sent > 0 {
        let total_comms = actions.emails_sent + actions.messages_sent;
        signals.push(Signal {
            principle: Principle::Transparency,
            severity: if total_comms > 20 {
                Severity::Concern
            } else {
                Severity::Info
            },
            title: "AI-sent communications".to_string(),
            detail: format!(
                "{} message(s) and {} email(s) sent by AI agents. \
                Do recipients know these came from an AI? \
                Simulated communication can erode trust when discovered.",
                actions.messages_sent, actions.emails_sent
            ),
            evidence: format!(
                "Channels: {}",
                if actions.channels_used.is_empty() {
                    "unknown".to_string()
                } else {
                    actions.channels_used.join(", ")
                }
            ),
        });
    }

    if actions.approvals_denied > 0 {
        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Info,
            title: "Agent actions denied by human".to_string(),
            detail: format!(
                "{} action(s) were requested by the AI but denied by a human. \
                This is the approval system working correctly.",
                actions.approvals_denied
            ),
            evidence: format!(
                "{} denied, {} approved of {} requested",
                actions.approvals_denied,
                actions.approvals_granted,
                actions.approvals_requested
            ),
        });
    }

    let approval_rate = if actions.approvals_requested > 0 {
        actions.approvals_granted as f64 / actions.approvals_requested as f64
    } else {
        0.0
    };

    if approval_rate > 0.95 && actions.approvals_requested > 10 {
        signals.push(Signal {
            principle: Principle::HumanAgency,
            severity: Severity::Info,
            title: "Near-automatic approval pattern".to_string(),
            detail: format!(
                "{:.0}% of {} approval requests were granted. \
                If approvals are always granted, the human-in-the-loop \
                may be rubber-stamping rather than reviewing.",
                approval_rate * 100.0,
                actions.approvals_requested
            ),
            evidence: format!(
                "{} approved, {} denied",
                actions.approvals_granted, actions.approvals_denied
            ),
        });
    }
}

fn detect_prompt_injection_risk(summary: &RepoSummary, signals: &mut Vec<Signal>) {
    let injection_patterns: &[(&str, &str)] = &[
        ("ignore previous instructions", "instruction override"),
        ("ignore all prior", "instruction override"),
        ("disregard all prior", "instruction override"),
        ("you are now", "role reassignment"),
        ("system prompt", "system prompt reference"),
        ("<system>", "system tag injection"),
        ("Human:", "conversation injection"),
        ("Assistant:", "conversation injection"),
        ("\u{200b}", "zero-width character"),
        ("\u{200c}", "zero-width character"),
        ("\u{200d}", "zero-width character"),
        ("\u{feff}", "zero-width character"),
    ];

    let mut flagged_prs: Vec<(u64, String, String)> = Vec::new();

    for pr in &summary.pull_requests {
        if let Some(ref body) = pr.body {
            let lower = body.to_lowercase();
            for (pattern, category) in injection_patterns {
                if lower.contains(&pattern.to_lowercase()) {
                    flagged_prs.push((pr.number, pr.title.clone(), category.to_string()));
                    break;
                }
            }
        }
    }

    if !flagged_prs.is_empty() {
        let pr_list: Vec<String> = flagged_prs
            .iter()
            .map(|(num, title, cat)| format!("#{} \"{}\" ({})", num, truncate(title, 40), cat))
            .collect();

        signals.push(Signal {
            principle: Principle::Security,
            severity: Severity::Warning,
            title: "Potential prompt injection in PR descriptions".to_string(),
            detail: format!(
                "{} PR(s) contain patterns commonly used in prompt injection attacks. \
                Review these PR descriptions before allowing AI tools to process them.",
                flagged_prs.len()
            ),
            evidence: pr_list.join("; "),
        });
    }
}
