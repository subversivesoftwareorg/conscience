use crate::ai_tools::claude_code::ClaudeCodeParser;
use crate::ai_tools::parser::AiToolParser;
use crate::error::Result;
use crate::ethics::models::*;
use crate::interval::Interval;
use crate::pipeline;
use crate::project::{resolve_project, ProjectScope};
use std::collections::HashSet;
use std::path::PathBuf;

pub async fn analyze_all_projects(interval: &Interval) -> Result<MultiProjectAnalysis> {
    let parser = ClaudeCodeParser::new();
    if !parser.detect() {
        eprintln!("No Claude Code data found at {}", parser.data_path());
        return Ok(MultiProjectAnalysis {
            projects: Vec::new(),
            outlier_signals: Vec::new(),
            total_projects: 0,
            total_sessions: 0,
            total_output_tokens: 0,
        });
    }

    let project_dirs = discover_projects(&parser);
    eprintln!("Found {} projects with Claude Code data", project_dirs.len());

    let mut projects = Vec::new();
    let mut total_sessions = 0u64;
    let mut total_output_tokens = 0u64;

    for (project_path, _dir_name) in &project_dirs {
        let path = PathBuf::from(project_path);
        // The directory may no longer exist (deleted checkout); still analyze
        // its sessions, but only load a manifest when it does.
        let scope = match resolve_project(Some(&path)) {
            Ok(s) => s,
            Err(_) => ProjectScope::from_root(path.clone()),
        };
        let project_name = scope.display_name();
        let repo = scope.manifest.as_ref().and_then(|m| m.github.repo.clone());

        // Same pipeline as examine, so every row is a real assessment with
        // coverage. Snapshots are not persisted here: that would write into
        // other repositories' directories.
        let collected = pipeline::collect(pipeline::CollectRequest {
            scope: &scope,
            interval: *interval,
            repo: repo.as_deref(),
            pr: None,
        })
        .await?;

        let Some(ai_summary) = collected.ai.as_ref() else {
            continue;
        };

        let ai_human_ratio = if ai_summary.total_turns.human > 0 {
            ai_summary.total_turns.assistant as f64 / ai_summary.total_turns.human as f64
        } else {
            0.0
        };

        total_sessions += ai_summary.session_count;
        total_output_tokens += ai_summary.total_tokens.output;

        let session_count = ai_summary.session_count;
        let output_tokens = ai_summary.total_tokens.output;
        let agent_dispatches: u64 = ai_summary
            .sessions
            .iter()
            .map(|s| s.agent_dispatches.len() as u64)
            .sum();
        let skill_invocations: u64 = ai_summary
            .sessions
            .iter()
            .map(|s| s.skill_invocations.len() as u64)
            .sum();

        let snapshot = pipeline::analyze(&scope, collected);

        projects.push(ProjectAnalysis {
            project_path: project_path.clone(),
            project_name: Some(project_name),
            snapshot_id: Some(snapshot.snapshot_id.clone()),
            coverage: Some(snapshot.coverage.clone()),
            analysis: snapshot.analysis,
            session_count,
            total_output_tokens: output_tokens,
            ai_human_ratio,
            agent_dispatches,
            skill_invocations,
        });
    }

    let outlier_signals = detect_cross_project_outliers(&projects);

    Ok(MultiProjectAnalysis {
        total_projects: projects.len(),
        total_sessions,
        total_output_tokens,
        outlier_signals,
        projects,
    })
}

fn discover_projects(parser: &ClaudeCodeParser) -> Vec<(String, String)> {
    let projects_dir = PathBuf::from(parser.data_path());
    let mut result = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    if let Ok(entries) = std::fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if !entry.path().is_dir() {
                continue;
            }

            let has_sessions = std::fs::read_dir(entry.path())
                .into_iter()
                .flatten()
                .flatten()
                .any(|f| f.path().extension().is_some_and(|e| e == "jsonl"));

            if !has_sessions {
                continue;
            }

            let project_path = parser.project_root_for_dir(&dir_name);
            if seen_paths.insert(project_path.clone()) {
                result.push((project_path, dir_name));
            }
        }
    }

    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn detect_cross_project_outliers(projects: &[ProjectAnalysis]) -> Vec<Signal> {
    let mut signals = Vec::new();

    if projects.len() < 2 {
        return signals;
    }

    // Token concentration: does one project use a disproportionate share?
    let total_tokens: u64 = projects.iter().map(|p| p.total_output_tokens).sum();
    if total_tokens > 0 {
        for project in projects {
            let share = project.total_output_tokens as f64 / total_tokens as f64;
            if share > 0.5 && projects.len() > 2 {
                signals.push(Signal {
                    id: "multi_token_concentration".to_string(),
                    principle: Principle::EnvironmentalCost,
                    severity: Severity::Info,
                    title: "Token concentration across projects".to_string(),
                    detail: format!(
                        "\"{}\" consumes {:.0}% of all AI tokens across {} projects.",
                        project.project_name.as_deref().unwrap_or("unknown"),
                        share * 100.0,
                        projects.len()
                    ),
                    evidence: format!(
                        "{}K tokens out of {}K total",
                        project.total_output_tokens / 1_000,
                        total_tokens / 1_000
                    ),
                });
            }
        }
    }

    // Worst AI:Human ratio
    if let Some(worst) = projects
        .iter()
        .filter(|p| p.ai_human_ratio > 3.0)
        .max_by(|a, b| a.ai_human_ratio.partial_cmp(&b.ai_human_ratio).unwrap())
    {
        signals.push(Signal {
            id: "multi_ai_turn_ratio_highest".to_string(),
            principle: Principle::HumanAgency,
            severity: Severity::Concern,
            title: "Highest AI dependency".to_string(),
            detail: format!(
                "\"{}\" has an AI:Human ratio of {:.1}:1 — the highest across all projects.",
                worst.project_name.as_deref().unwrap_or("unknown"),
                worst.ai_human_ratio
            ),
            evidence: format!("{} sessions", worst.session_count),
        });
    }

    // Most security warnings
    for project in projects {
        let security_warnings = project
            .analysis
            .signals
            .iter()
            .filter(|s| s.principle == Principle::Security && s.severity >= Severity::Warning)
            .count();

        if security_warnings > 3 {
            signals.push(Signal {
                id: "multi_security_warnings".to_string(),
                principle: Principle::Security,
                severity: Severity::Warning,
                title: "Multiple security warnings".to_string(),
                detail: format!(
                    "\"{}\" has {} security warnings — more than any other project. Investigate.",
                    project.project_name.as_deref().unwrap_or("unknown"),
                    security_warnings
                ),
                evidence: format!("Project: {}", project.project_path),
            });
        }
    }

    // Heaviest orchestration
    if let Some(heaviest) = projects
        .iter()
        .filter(|p| p.agent_dispatches > 10)
        .max_by_key(|p| p.agent_dispatches)
    {
        signals.push(Signal {
            id: "multi_agent_orchestration_highest".to_string(),
            principle: Principle::HumanAgency,
            severity: Severity::Info,
            title: "Highest agent orchestration".to_string(),
            detail: format!(
                "\"{}\" dispatched {} agents and {} skills — the heaviest orchestration across projects. \
                Heavy orchestration concentrates work through a single seat and compounds token costs.",
                heaviest.project_name.as_deref().unwrap_or("unknown"),
                heaviest.agent_dispatches,
                heaviest.skill_invocations,
            ),
            evidence: format!(
                "{} sessions, {}K output tokens",
                heaviest.session_count,
                heaviest.total_output_tokens / 1_000,
            ),
        });
    }

    signals
}
