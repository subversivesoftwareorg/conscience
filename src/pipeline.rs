//! The shared analysis pipeline: collect, then analyze into a snapshot.
//!
//! Every command that assesses a project goes through here so they agree
//! on scope, interval, coverage accounting, and the shape of the result.
//! `collect` never turns a source failure into silence: a GitHub error is
//! recorded as `Failed` with the message, not dropped as "no GitHub data".

use crate::ai_tools::models::AiUsageSummary;
use crate::analysis::energy;
use crate::error::Result;
use crate::ethics;
use crate::ethics::manifest::Thresholds;
use crate::github::models::RepoSummary;
use crate::ingest;
use crate::interval::Interval;
use crate::project::ProjectScope;
use crate::snapshot::*;
use chrono::Utc;

/// What to collect. Exactly one of `repo` or `pr` may be set; with `pr`
/// the interval is replaced by the pull request's own lifetime.
pub struct CollectRequest<'a> {
    pub scope: &'a ProjectScope,
    pub interval: Interval,
    pub repo: Option<&'a str>,
    pub pr: Option<&'a str>,
}

pub struct Collected {
    pub github: Option<RepoSummary>,
    pub ai: Option<AiUsageSummary>,
    pub coverage: Coverage,
    /// The interval actually applied (differs from the request for `pr`).
    pub interval: Interval,
}

pub async fn collect(req: CollectRequest<'_>) -> Result<Collected> {
    let mut coverage = Coverage {
        manifest_found: req.scope.manifest.is_some(),
        ..Default::default()
    };
    let mut interval = req.interval;

    // GitHub first: for a PR it defines the interval the AI sessions use.
    let github = if let Some(pr) = req.pr {
        match ingest::github::ingest_pr(pr).await {
            Ok(s) => {
                interval = Interval::between(s.period_start, s.period_end);
                coverage.github_commits = s.commits.len() as u64;
                coverage.github_pull_requests = s.pull_requests.len() as u64;
                coverage.sources.push(SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Collected,
                    detail: format!("PR with {} commits", s.commits.len()),
                });
                Some(s)
            }
            Err(e) => {
                coverage.sources.push(SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Failed,
                    detail: e.to_string(),
                });
                None
            }
        }
    } else if let Some(repo) = req.repo {
        match ingest::github::ingest_github(repo, &interval).await {
            Ok(s) => {
                coverage.github_commits = s.commits.len() as u64;
                coverage.github_pull_requests = s.pull_requests.len() as u64;
                coverage.sources.push(SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Collected,
                    detail: format!("{} commits, {} PRs", s.commits.len(), s.pull_requests.len()),
                });
                Some(s)
            }
            Err(e) => {
                eprintln!("Warning: GitHub collection failed: {}", e);
                coverage.sources.push(SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Failed,
                    detail: e.to_string(),
                });
                None
            }
        }
    } else {
        coverage.sources.push(SourceCoverage {
            source: "github".into(),
            status: SourceStatus::Skipped,
            detail: "no --repo given".into(),
        });
        None
    };

    let ai = match ingest::ai::ingest_claude_code(Some(req.scope), Some(&interval)) {
        Ok(s) => {
            coverage.ai_sessions_in_range = s.session_count;
            coverage.ai_sessions_undated = s.undated_sessions;
            let status = if s.session_count > 0 {
                SourceStatus::Collected
            } else {
                SourceStatus::Unavailable
            };
            let detail = if s.session_count > 0 {
                let mut d = format!("{} sessions", s.session_count);
                if s.undated_sessions > 0 {
                    d.push_str(&format!(", {} undated", s.undated_sessions));
                }
                d
            } else {
                "no sessions in range for this project".into()
            };
            coverage.sources.push(SourceCoverage {
                source: "claude_code".into(),
                status,
                detail,
            });
            if s.session_count > 0 { Some(s) } else { None }
        }
        Err(e) => {
            coverage.sources.push(SourceCoverage {
                source: "claude_code".into(),
                status: SourceStatus::Failed,
                detail: e.to_string(),
            });
            None
        }
    };

    Ok(Collected {
        github,
        ai,
        coverage,
        interval,
    })
}

/// Turn collected data into a snapshot: signals, scorecard, reflections,
/// structured metrics, and the analyzer/config that produced them.
pub fn analyze(scope: &ProjectScope, collected: Collected) -> Snapshot {
    let manifest = scope.manifest.as_ref();
    let thresholds: Thresholds = manifest.map(|m| m.thresholds.clone()).unwrap_or_default();

    let analysis = ethics::analyze(collected.github.as_ref(), collected.ai.as_ref(), manifest);

    let mut metrics = Vec::new();
    if let Some(ai) = &collected.ai {
        metrics.extend(ai_metrics(ai));
        metrics.extend(energy_metrics(ai, &thresholds));
    }
    if let Some(gh) = &collected.github {
        metrics.extend(github_metrics(gh));
    }

    Snapshot {
        schema_version: SCHEMA_VERSION,
        snapshot_id: stamp_id(Utc::now()),
        project: ProjectIdentity {
            id: format!("{:016x}", fnv1a(scope.root.to_string_lossy().as_bytes())),
            name: scope.display_name(),
            root: scope.root.to_string_lossy().to_string(),
            github_repo: collected
                .github
                .as_ref()
                .map(|g| format!("{}/{}", g.owner, g.repo))
                .or_else(|| manifest.and_then(|m| m.github.repo.clone())),
            worktrees: scope
                .worktrees
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        },
        interval: collected.interval,
        coverage: collected.coverage,
        metrics,
        analyzer: AnalyzerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            config_fingerprint: fingerprint(&thresholds),
            thresholds_source: if manifest.is_some() {
                "manifest"
            } else {
                "defaults"
            }
            .into(),
        },
        analysis,
    }
}

/// Collect and analyze in one step.
pub async fn run(req: CollectRequest<'_>) -> Result<Snapshot> {
    let scope = req.scope;
    let collected = collect(req).await?;
    Ok(analyze(scope, collected))
}

fn ai_metrics(ai: &AiUsageSummary) -> Vec<Metric> {
    let human = ai.total_turns.human as f64;
    let assistant = ai.total_turns.assistant as f64;
    let mut m = vec![
        Metric::observed("ai.sessions", ai.session_count as f64, "sessions"),
        Metric::observed("ai.human_turns", human, "turns"),
        Metric::observed("ai.assistant_turns", assistant, "turns"),
        Metric::observed("ai.input_tokens", ai.total_tokens.input as f64, "tokens"),
        Metric::observed("ai.output_tokens", ai.total_tokens.output as f64, "tokens"),
        Metric::observed(
            "ai.cache_read_tokens",
            ai.total_tokens.cache_read as f64,
            "tokens",
        ),
        Metric::observed(
            "ai.cache_creation_tokens",
            ai.total_tokens.cache_creation as f64,
            "tokens",
        ),
        Metric::observed(
            "ai.files_touched",
            ai.files_touched_count as f64,
            "operations",
        ),
        Metric::observed(
            "ai.bash_commands",
            ai.all_bash_commands.len() as f64,
            "commands",
        ),
    ];
    if human > 0.0 {
        m.push(Metric::ratio(
            "ai.turn_ratio",
            assistant / human,
            "assistant_turns",
            "human_turns",
        ));
    }
    m
}

fn energy_metrics(ai: &AiUsageSummary, thresholds: &Thresholds) -> Vec<Metric> {
    let est = energy::estimate_total_energy(&ai.sessions, &thresholds.energy);
    let (low, high) = est.uncertainty_range;
    let mut m = vec![Metric::estimated(
        "energy.wh",
        est.total_wh,
        "Wh",
        low,
        high,
    )];
    if let Some(co2) = est.co2_kg {
        // Scale the CO2 bounds by the same relative spread as the energy.
        let (lo, hi) = if est.total_wh > 0.0 {
            (co2 * low / est.total_wh, co2 * high / est.total_wh)
        } else {
            (co2, co2)
        };
        m.push(Metric::estimated("energy.co2_kg", co2, "kg", lo, hi));
    }
    if let Some(water) = est.water_liters {
        let (lo, hi) = if est.total_wh > 0.0 {
            (water * low / est.total_wh, water * high / est.total_wh)
        } else {
            (water, water)
        };
        m.push(Metric::estimated("energy.water_liters", water, "L", lo, hi));
    }
    m
}

fn github_metrics(gh: &RepoSummary) -> Vec<Metric> {
    let merged: Vec<_> = gh
        .pull_requests
        .iter()
        .filter(|p| p.merged_at.is_some())
        .collect();
    let mut m = vec![
        Metric::observed("github.commits", gh.commits.len() as f64, "commits"),
        Metric::observed("github.pull_requests", gh.pull_requests.len() as f64, "prs"),
        Metric::observed("github.prs_merged", merged.len() as f64, "prs"),
    ];
    if !merged.is_empty() {
        let comments: u64 = merged.iter().map(|p| p.review_comments).sum();
        m.push(Metric::ratio(
            "github.review_comments_per_merged_pr",
            comments as f64 / merged.len() as f64,
            "comments",
            "merged_prs",
        ));
        let mut hours: Vec<f64> = merged
            .iter()
            .filter_map(|p| p.time_to_merge_hours)
            .collect();
        if !hours.is_empty() {
            hours.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = hours.len() / 2;
            let median = if hours.len() % 2 == 0 {
                (hours[mid - 1] + hours[mid]) / 2.0
            } else {
                hours[mid]
            };
            m.push(Metric::observed(
                "github.median_time_to_merge_hours",
                median,
                "hours",
            ));
        }
    }
    m
}
