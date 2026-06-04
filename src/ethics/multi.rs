use crate::ai_tools::claude_code::{decode_project_dir, ClaudeCodeParser};
use crate::ai_tools::parser::AiToolParser;
use crate::error::Result;
use crate::ethics;
use crate::ethics::manifest::Manifest;
use crate::ethics::models::*;
use crate::ingest;
use std::collections::HashSet;
use std::path::PathBuf;

pub async fn analyze_all_projects(days: u32) -> Result<MultiProjectAnalysis> {
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
        let manifest = Manifest::load(&path);

        let project_name = manifest
            .as_ref()
            .map(|m| m.project.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });

        let ai_summary = match ingest::ai::ingest_claude_code(Some(&path)) {
            Ok(s) if s.session_count > 0 => s,
            _ => continue,
        };

        let github_summary = if let Some(ref m) = manifest {
            if let Some(ref repo) = m.github.repo {
                match ingest::github::ingest_github(repo, days).await {
                    Ok(s) => Some(s),
                    Err(e) => {
                        eprintln!("  Warning: failed to fetch GitHub data for {}: {}", repo, e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let analysis = ethics::analyze(
            github_summary.as_ref(),
            Some(&ai_summary),
            manifest.as_ref(),
        );

        let ai_human_ratio = if ai_summary.total_turns.human > 0 {
            ai_summary.total_turns.assistant as f64 / ai_summary.total_turns.human as f64
        } else {
            0.0
        };

        total_sessions += ai_summary.session_count;
        total_output_tokens += ai_summary.total_tokens.output;

        projects.push(ProjectAnalysis {
            project_path: project_path.clone(),
            project_name: Some(project_name),
            analysis,
            session_count: ai_summary.session_count,
            total_output_tokens: ai_summary.total_tokens.output,
            ai_human_ratio,
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

            let project_path = decode_project_dir(&dir_name);
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

    signals
}
