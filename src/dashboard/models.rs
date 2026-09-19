use crate::ethics::models::EthicalAnalysis;
use crate::snapshot::{AnalyzerInfo, Coverage, Metric, Snapshot};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What `push` sends. Version 1 shape kept intact for the existing
/// dashboard; snapshot identity, analyzer, coverage, and metrics are
/// additive. The allowlisted, sanitized export is roadmap #378.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPayload {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub project: ProjectIdentifier,
    pub analysis: EthicalAnalysis,
    pub stats: AnalysisStats,
    #[serde(default)]
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub analyzer: Option<AnalyzerInfo>,
    #[serde(default)]
    pub coverage: Option<Coverage>,
    #[serde(default)]
    pub metrics: Vec<Metric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIdentifier {
    pub name: String,
    pub github_repo: Option<String>,
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisStats {
    pub signal_count: usize,
    pub warning_count: usize,
    pub concern_count: usize,
    pub principles_covered: Vec<String>,
    pub ai_sessions: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub period_days: u32,
    /// Explicit bounds of the interval every number above was computed over.
    #[serde(default)]
    pub period_start: Option<DateTime<Utc>>,
    #[serde(default)]
    pub period_end: Option<DateTime<Utc>>,
    /// AI sessions with no timestamp, excluded from all totals.
    #[serde(default)]
    pub undated_sessions: u64,
}

impl DashboardPayload {
    /// Build the payload from an inspected snapshot. Nothing is recomputed.
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let analysis = snapshot.analysis.clone();
        let signal_count = analysis.signals.len();
        let warning_count = snapshot.signal_count(crate::ethics::models::Severity::Warning);
        let concern_count = snapshot.signal_count(crate::ethics::models::Severity::Concern);

        let mut principles: Vec<String> = analysis
            .scorecard
            .iter()
            .filter(|d| !d.auto_signals.is_empty())
            .map(|d| d.principle.name().to_string())
            .collect();
        principles.dedup();

        let ai_sessions = snapshot.metric("ai.sessions").map(|m| m.value as u64);
        let total_output_tokens = snapshot.metric("ai.output_tokens").map(|m| m.value as u64);

        Self {
            version: "1.1".to_string(),
            timestamp: snapshot.interval.collected_at,
            project: ProjectIdentifier {
                name: snapshot.project.name.clone(),
                github_repo: snapshot.project.github_repo.clone(),
                project_path: Some(snapshot.project.root.clone()),
            },
            stats: AnalysisStats {
                signal_count,
                warning_count,
                concern_count,
                principles_covered: principles,
                ai_sessions,
                total_output_tokens,
                period_days: snapshot.interval.days(),
                period_start: Some(snapshot.interval.start),
                period_end: Some(snapshot.interval.end),
                undated_sessions: snapshot.coverage.ai_sessions_undated,
            },
            analysis,
            snapshot_id: Some(snapshot.snapshot_id.clone()),
            project_id: Some(snapshot.project.id.clone()),
            analyzer: Some(snapshot.analyzer.clone()),
            coverage: Some(snapshot.coverage.clone()),
            metrics: snapshot.metrics.clone(),
        }
    }
}
