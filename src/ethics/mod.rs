pub mod manifest;
pub mod models;
pub mod multi;
pub mod reflection;
pub mod report;
pub mod scorecard;
pub mod session;
pub mod signals;

use crate::ai_tools::models::AiUsageSummary;
use crate::github::models::RepoSummary;
use manifest::Manifest;
use models::EthicalAnalysis;

/// Run the full ethical analysis: signals -> scorecard -> reflections.
/// If a manifest (conscience.yaml) is provided, it enriches the analysis
/// with human-provided context about project purpose, value, and team.
pub fn analyze(
    github: Option<&RepoSummary>,
    ai: Option<&AiUsageSummary>,
    manifest: Option<&Manifest>,
) -> EthicalAnalysis {
    let thresholds = manifest.map(|m| &m.thresholds);
    let mut all_signals = Vec::new();

    if let Some(gh) = github {
        all_signals.extend(signals::detect_github_signals(gh, thresholds));
    }

    if let Some(ai_data) = ai {
        all_signals.extend(signals::detect_ai_signals(ai_data, thresholds));
    }

    if let Some(m) = manifest {
        all_signals.extend(signals::detect_manifest_signals(m));
    }

    let scorecard = scorecard::build_scorecard(&all_signals);
    let reflections = reflection::generate_reflections(github, ai, manifest);

    EthicalAnalysis {
        signals: all_signals,
        scorecard,
        reflections,
    }
}
