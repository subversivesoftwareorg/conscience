use crate::ethics::models::EthicalAnalysis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPayload {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub project: ProjectIdentifier,
    pub analysis: EthicalAnalysis,
    pub stats: AnalysisStats,
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
}

impl DashboardPayload {
    pub fn new(
        project_name: String,
        github_repo: Option<String>,
        project_path: Option<String>,
        analysis: EthicalAnalysis,
        ai_sessions: Option<u64>,
        total_output_tokens: Option<u64>,
        period_days: u32,
    ) -> Self {
        let signal_count = analysis.signals.len();
        let warning_count = analysis
            .signals
            .iter()
            .filter(|s| s.severity == crate::ethics::models::Severity::Warning)
            .count();
        let concern_count = analysis
            .signals
            .iter()
            .filter(|s| s.severity == crate::ethics::models::Severity::Concern)
            .count();

        let mut principles: Vec<String> = analysis
            .scorecard
            .iter()
            .filter(|d| !d.auto_signals.is_empty())
            .map(|d| d.principle.name().to_string())
            .collect();
        principles.dedup();

        Self {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            project: ProjectIdentifier {
                name: project_name,
                github_repo,
                project_path,
            },
            stats: AnalysisStats {
                signal_count,
                warning_count,
                concern_count,
                principles_covered: principles,
                ai_sessions,
                total_output_tokens,
                period_days,
            },
            analysis,
        }
    }
}
