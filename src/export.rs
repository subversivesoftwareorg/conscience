//! The allowlisted export: what `push` is permitted to send.
//!
//! The dashboard stores whatever it receives verbatim, so the wire model is
//! built field by field from the local snapshot. Nothing crosses unless it
//! is named in [`SnapshotExport::from_snapshot`]. Signal evidence goes
//! through a per-signal policy table; a signal whose id has no entry is
//! redacted, and a test insists every emitted id has one.
//!
//! Kept out by default: the project's filesystem path and worktrees, raw
//! shell commands, file paths, PR bodies and titles inside signals, session
//! ids, contributor names inside signals, and the text of collection errors.

use crate::ethics::models::{Principle, Severity};
use crate::snapshot::{AnalyzerInfo, Metric, Snapshot, SourceStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const EXPORT_VERSION: &str = "2";

/// Wire payload. Field names and nesting match what the dashboard's ingest
/// endpoint requires; additional fields are ignored by older servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotExport {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub project: ExportProject,
    pub analysis: ExportAnalysis,
    pub stats: ExportStats,
    pub snapshot_id: String,
    pub project_id: String,
    pub interval: ExportInterval,
    pub coverage: ExportCoverage,
    pub metrics: Vec<Metric>,
    pub analyzer: AnalyzerInfo,
    /// What was removed or replaced on the way out, for the record.
    pub sanitization: Sanitization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProject {
    pub name: String,
    pub github_repo: Option<String>,
    /// Always `None`: a local checkout path is personal to the machine.
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportAnalysis {
    pub signals: Vec<ExportSignal>,
    pub scorecard: Vec<ExportDimension>,
    pub reflections: Vec<ExportReflection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSignal {
    pub id: String,
    pub principle: Principle,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDimension {
    pub principle: Principle,
    pub auto_signals: Vec<ExportSignal>,
    pub needs_human_input: bool,
    pub human_assessment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReflection {
    pub principle: Principle,
    pub data_context: String,
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStats {
    pub signal_count: usize,
    pub warning_count: usize,
    pub concern_count: usize,
    pub principles_covered: Vec<String>,
    pub ai_sessions: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub period_days: u32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub undated_sessions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInterval {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCoverage {
    pub sources: Vec<ExportSource>,
    pub ai_sessions_in_range: u64,
    pub ai_sessions_undated: u64,
    pub github_commits: u64,
    pub github_pull_requests: u64,
    pub manifest_found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSource {
    pub source: String,
    pub status: SourceStatus,
    /// Counts for collected sources; a fixed phrase for failures, never the
    /// error text, which can embed paths or tokens.
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sanitization {
    pub evidence_replaced: u64,
    pub evidence_redacted: u64,
    pub signals_rewritten: u64,
    pub project_path_omitted: bool,
    pub failure_details_omitted: u64,
}

impl Sanitization {
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.evidence_replaced > 0 {
            parts.push(format!(
                "evidence replaced with counts on {} signal(s)",
                self.evidence_replaced
            ));
        }
        if self.evidence_redacted > 0 {
            parts.push(format!(
                "evidence removed on {} signal(s)",
                self.evidence_redacted
            ));
        }
        if self.signals_rewritten > 0 {
            parts.push(format!(
                "identifying text removed from {} signal(s)",
                self.signals_rewritten
            ));
        }
        if self.project_path_omitted {
            parts.push("project path omitted".into());
        }
        if self.failure_details_omitted > 0 {
            parts.push(format!(
                "{} collection error message(s) omitted",
                self.failure_details_omitted
            ));
        }
        if parts.is_empty() {
            "nothing needed sanitizing".into()
        } else {
            parts.join(", ")
        }
    }
}

/// What may leave the machine for a given signal id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidencePolicy {
    /// Evidence is counts, ratios, or config key names only.
    Keep,
    /// Evidence lists items (paths, commands, PRs); send only how many.
    Count(&'static str),
    /// Evidence is not needed remotely; send an empty string.
    Redact,
    /// Title or detail embed a name or id; substitute fixed wording.
    Rewrite {
        title: Option<&'static str>,
        detail: &'static str,
        evidence: Option<&'static str>,
    },
}

/// The policy table. Every signal id the detectors can emit must appear
/// here; `tests/export_test.rs` enforces it. Unknown ids are redacted.
pub const POLICIES: &[(&str, EvidencePolicy)] = &[
    // Manifest: team-authored context, meant to be shared.
    ("manifest_review_stale", EvidencePolicy::Keep),
    ("manifest_no_beneficiaries", EvidencePolicy::Keep),
    ("manifest_value_documented", EvidencePolicy::Keep),
    ("manifest_ai_sentiment", EvidencePolicy::Keep),
    ("manifest_juniors_no_learning_goals", EvidencePolicy::Keep),
    // GitHub: counts and percentages, except the single-author case.
    (
        "github_single_contributor",
        EvidencePolicy::Rewrite {
            title: None,
            detail: "All commits in the period come from one contributor.",
            evidence: Some("all commits from a single author"),
        },
    ),
    (
        "github_contribution_concentration_high",
        EvidencePolicy::Keep,
    ),
    (
        "github_contribution_concentration_moderate",
        EvidencePolicy::Keep,
    ),
    ("github_contributions_distributed", EvidencePolicy::Keep),
    ("github_review_engagement_low", EvidencePolicy::Keep),
    ("github_merge_time_fast", EvidencePolicy::Keep),
    ("github_velocity_baseline", EvidencePolicy::Keep),
    // AI usage: counts and token figures.
    ("ai_turn_ratio_high", EvidencePolicy::Keep),
    ("ai_turn_ratio_moderate", EvidencePolicy::Keep),
    ("ai_turn_ratio_balanced", EvidencePolicy::Keep),
    ("ai_new_file_ratio_high", EvidencePolicy::Keep),
    ("ai_token_consumption_high", EvidencePolicy::Keep),
    ("ai_cache_efficiency_good", EvidencePolicy::Keep),
    ("ai_bash_volume_high", EvidencePolicy::Keep),
    ("ai_agent_orchestration_heavy", EvidencePolicy::Keep),
    // Per-session signals name the session id in `detail`.
    (
        "ai_tokens_per_file_high",
        EvidencePolicy::Rewrite {
            title: None,
            detail: "A session used a large number of output tokens relative to the files it touched.",
            evidence: None,
        },
    ),
    (
        "ai_tokens_per_turn_high",
        EvidencePolicy::Rewrite {
            title: None,
            detail: "A session averaged an unusually high number of output tokens per turn.",
            evidence: None,
        },
    ),
    (
        "ai_session_length_extreme",
        EvidencePolicy::Rewrite {
            title: None,
            detail: "A session ran far longer than the configured maximum.",
            evidence: None,
        },
    ),
    // Security: evidence lists paths and commands.
    (
        "security_sensitive_file_write",
        EvidencePolicy::Count("file(s)"),
    ),
    (
        "security_sensitive_file_read",
        EvidencePolicy::Count("file(s)"),
    ),
    (
        "security_network_exfiltration",
        EvidencePolicy::Count("command(s)"),
    ),
    (
        "security_encoding_obfuscation",
        EvidencePolicy::Count("command(s)"),
    ),
    (
        "security_credential_access",
        EvidencePolicy::Count("command(s)"),
    ),
    (
        "security_prompt_injection_pr",
        EvidencePolicy::Count("pull request(s)"),
    ),
    // Agent actions: counts, except the channel list.
    ("agent_actions_unapproved", EvidencePolicy::Keep),
    (
        "agent_communications_sent",
        EvidencePolicy::Count("channel(s)"),
    ),
    ("agent_actions_denied", EvidencePolicy::Keep),
    ("agent_approval_near_automatic", EvidencePolicy::Keep),
    // Authorship: the per-contributor signal names the person.
    ("authorship_ai_correlation_very_high", EvidencePolicy::Keep),
    ("authorship_ai_correlation_majority", EvidencePolicy::Keep),
    (
        "authorship_contributor_near_total_ai",
        EvidencePolicy::Rewrite {
            title: Some("A contributor's commits are almost all AI-correlated"),
            detail: "More than 90% of one contributor's commits correlate with AI sessions.",
            evidence: None,
        },
    ),
    // Cross-project outliers are never pushed (push is per project), but
    // they name other projects, so redact if one ever gets through.
    ("multi_token_concentration", EvidencePolicy::Redact),
    ("multi_ai_turn_ratio_highest", EvidencePolicy::Redact),
    ("multi_security_warnings", EvidencePolicy::Redact),
    ("multi_agent_orchestration_highest", EvidencePolicy::Redact),
];

pub fn policy_for(id: &str) -> EvidencePolicy {
    POLICIES
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, p)| *p)
        .unwrap_or(EvidencePolicy::Redact)
}

fn count_items(evidence: &str) -> usize {
    if evidence.trim().is_empty() {
        return 0;
    }
    if evidence.contains('\n') {
        evidence.lines().filter(|l| !l.trim().is_empty()).count()
    } else if evidence.contains("; ") {
        evidence.split("; ").count()
    } else {
        evidence.split(", ").count()
    }
}

fn export_signal(s: &crate::ethics::models::Signal, san: &mut Sanitization) -> ExportSignal {
    let (title, detail, evidence) = match policy_for(&s.id) {
        EvidencePolicy::Keep => (s.title.clone(), s.detail.clone(), s.evidence.clone()),
        EvidencePolicy::Count(noun) => {
            san.evidence_replaced += 1;
            (
                s.title.clone(),
                s.detail.clone(),
                format!(
                    "{} {} matched; details retained locally",
                    count_items(&s.evidence),
                    noun
                ),
            )
        }
        EvidencePolicy::Redact => {
            san.evidence_redacted += 1;
            (s.title.clone(), s.detail.clone(), String::new())
        }
        EvidencePolicy::Rewrite {
            title,
            detail,
            evidence,
        } => {
            san.signals_rewritten += 1;
            (
                title.map(str::to_string).unwrap_or_else(|| s.title.clone()),
                detail.to_string(),
                evidence.map(str::to_string).unwrap_or_default(),
            )
        }
    };
    ExportSignal {
        id: s.id.clone(),
        principle: s.principle,
        severity: s.severity,
        title,
        detail,
        evidence,
    }
}

impl SnapshotExport {
    /// The only way to build an export. Every field is assigned here on
    /// purpose; there is deliberately no blanket conversion.
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let mut san = Sanitization {
            project_path_omitted: true,
            ..Default::default()
        };

        let signals: Vec<ExportSignal> = snapshot
            .analysis
            .signals
            .iter()
            .map(|s| export_signal(s, &mut san))
            .collect();

        // Scorecard entries repeat the signals; sanitize them the same way
        // but do not count them twice.
        let mut scratch = Sanitization::default();
        let scorecard: Vec<ExportDimension> = snapshot
            .analysis
            .scorecard
            .iter()
            .map(|d| ExportDimension {
                principle: d.principle,
                auto_signals: d
                    .auto_signals
                    .iter()
                    .map(|s| export_signal(s, &mut scratch))
                    .collect(),
                needs_human_input: d.needs_human_input,
                human_assessment: d.human_assessment.clone(),
            })
            .collect();

        let reflections: Vec<ExportReflection> = snapshot
            .analysis
            .reflections
            .iter()
            .map(|r| ExportReflection {
                principle: r.principle,
                data_context: r.data_context.clone(),
                question: r.question.clone(),
            })
            .collect();

        let sources: Vec<ExportSource> = snapshot
            .coverage
            .sources
            .iter()
            .map(|s| {
                let detail = match s.status {
                    SourceStatus::Failed => {
                        san.failure_details_omitted += 1;
                        "collection failed".to_string()
                    }
                    _ => s.detail.clone(),
                };
                ExportSource {
                    source: s.source.clone(),
                    status: s.status,
                    detail,
                }
            })
            .collect();

        let mut principles: Vec<String> = snapshot
            .analysis
            .scorecard
            .iter()
            .filter(|d| !d.auto_signals.is_empty())
            .map(|d| d.principle.name().to_string())
            .collect();
        principles.dedup();

        Self {
            version: EXPORT_VERSION.to_string(),
            timestamp: snapshot.interval.collected_at,
            project: ExportProject {
                name: snapshot.project.name.clone(),
                github_repo: snapshot.project.github_repo.clone(),
                project_path: None,
            },
            stats: ExportStats {
                signal_count: signals.len(),
                warning_count: snapshot.signal_count(Severity::Warning),
                concern_count: snapshot.signal_count(Severity::Concern),
                principles_covered: principles,
                ai_sessions: snapshot.metric("ai.sessions").map(|m| m.value as u64),
                total_output_tokens: snapshot.metric("ai.output_tokens").map(|m| m.value as u64),
                period_days: snapshot.interval.days(),
                period_start: snapshot.interval.start,
                period_end: snapshot.interval.end,
                undated_sessions: snapshot.coverage.ai_sessions_undated,
            },
            analysis: ExportAnalysis {
                signals,
                scorecard,
                reflections,
            },
            snapshot_id: snapshot.snapshot_id.clone(),
            project_id: snapshot.project.id.clone(),
            interval: ExportInterval {
                start: snapshot.interval.start,
                end: snapshot.interval.end,
                collected_at: snapshot.interval.collected_at,
            },
            coverage: ExportCoverage {
                sources,
                ai_sessions_in_range: snapshot.coverage.ai_sessions_in_range,
                ai_sessions_undated: snapshot.coverage.ai_sessions_undated,
                github_commits: snapshot.coverage.github_commits,
                github_pull_requests: snapshot.coverage.github_pull_requests,
                manifest_found: snapshot.coverage.manifest_found,
            },
            metrics: snapshot.metrics.clone(),
            analyzer: snapshot.analyzer.clone(),
            sanitization: san,
        }
    }
}
