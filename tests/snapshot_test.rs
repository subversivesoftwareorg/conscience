//! The snapshot is the unit of assessment: it must carry identity, interval,
//! coverage, metrics with units, and the analyzer that produced it; survive
//! a round trip to disk; keep `signals` at the top level of its JSON for
//! existing consumers; and be findable by id prefix.

use chrono::{DateTime, Duration, Utc};
use conscience::ai_tools::models::*;
use conscience::github::models::*;
use conscience::interval::Interval;
use conscience::pipeline::{self, Collected};
use conscience::project::ProjectScope;
use conscience::snapshot::*;
use std::collections::HashMap;
use std::path::PathBuf;

fn session(id: &str, start: DateTime<Utc>, output: u64, human: u64, assistant: u64) -> AiSession {
    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: id.into(),
        project_path: None,
        started_at: Some(start),
        ended_at: Some(start + Duration::hours(1)),
        model: Some("claude-fable-5-1".into()),
        work_categories: vec![],
        turns: TurnCounts {
            human,
            assistant,
            machine: 0,
            total: human + assistant,
        },
        tokens: TokenUsage {
            input: 1000,
            output,
            cache_creation: 0,
            cache_read: 0,
        },
        tools_used: HashMap::new(),
        files_touched: vec![],
        bash_commands: vec![],
        agent_actions: vec![],
        git_branch: None,
        interactions: vec![],
        agent_dispatches: vec![],
        skill_invocations: vec![],
    }
}

fn pr(number: u64, merged_hours: Option<f64>, comments: u64) -> PullRequestSummary {
    let now = Utc::now();
    PullRequestSummary {
        number,
        title: format!("PR {}", number),
        author: "alice".into(),
        state: "closed".into(),
        body: None,
        created_at: now - Duration::hours(48),
        merged_at: merged_hours
            .map(|h| now - Duration::hours(48) + Duration::minutes((h * 60.0) as i64)),
        closed_at: None,
        additions: Some(10),
        deletions: Some(2),
        changed_files: Some(1),
        review_comments: comments,
        time_to_merge_hours: merged_hours,
    }
}

fn collected() -> Collected {
    let iv = Interval::last_days(7);
    let ai = AiUsageSummary::from_sessions(
        AiTool::ClaudeCode,
        vec![
            session("a", Utc::now() - Duration::days(1), 5000, 10, 20),
            session("b", Utc::now() - Duration::days(2), 3000, 5, 5),
        ],
    )
    .restrict(&iv);
    let gh = RepoSummary {
        owner: "org".into(),
        repo: "repo".into(),
        period_start: iv.start,
        period_end: iv.end,
        commits: vec![],
        pull_requests: vec![pr(1, Some(2.0), 4), pr(2, Some(10.0), 0), pr(3, None, 1)],
    };
    Collected {
        github: Some(gh),
        ai: Some(ai),
        coverage: Coverage {
            sources: vec![
                SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Collected,
                    detail: "0 commits, 3 PRs".into(),
                },
                SourceCoverage {
                    source: "claude_code".into(),
                    status: SourceStatus::Collected,
                    detail: "2 sessions".into(),
                },
            ],
            ai_sessions_in_range: 2,
            ai_sessions_undated: 0,
            github_commits: 0,
            github_pull_requests: 3,
            manifest_found: false,
        },
        interval: iv,
    }
}

fn scope() -> ProjectScope {
    ProjectScope::from_root(PathBuf::from("/work/demo"))
}

#[test]
fn analyze_produces_identity_metrics_and_analyzer_info() {
    let snap = pipeline::analyze(&scope(), collected());

    assert_eq!(snap.schema_version, SCHEMA_VERSION);
    assert_eq!(snap.snapshot_id.len(), 23);
    assert_eq!(snap.project.name, "demo");
    assert_eq!(snap.project.id.len(), 16);
    assert_eq!(snap.project.github_repo.as_deref(), Some("org/repo"));

    // Observations carry units; ratios carry denominators.
    let sessions = snap.metric("ai.sessions").unwrap();
    assert_eq!(sessions.value, 2.0);
    assert_eq!(sessions.unit, "sessions");
    assert_eq!(sessions.kind, MetricKind::Observation);

    let ratio = snap.metric("ai.turn_ratio").unwrap();
    assert!((ratio.value - 25.0 / 15.0).abs() < 1e-9);
    assert_eq!(ratio.denominator.as_deref(), Some("human_turns"));

    assert_eq!(snap.metric("github.prs_merged").unwrap().value, 2.0);
    assert_eq!(
        snap.metric("github.median_time_to_merge_hours")
            .unwrap()
            .value,
        6.0
    );
    assert_eq!(
        snap.metric("github.review_comments_per_merged_pr")
            .unwrap()
            .value,
        2.0
    );

    // Energy is an estimate with a range around it.
    let wh = snap.metric("energy.wh").unwrap();
    assert_eq!(wh.kind, MetricKind::Estimate);
    let (lo, hi) = wh.uncertainty.unwrap();
    assert!(lo <= wh.value && wh.value <= hi);

    assert_eq!(snap.analyzer.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(snap.analyzer.thresholds_source, "defaults");
    assert_eq!(snap.analyzer.config_fingerprint.len(), 16);
    assert!(!snap.analysis.signals.is_empty());
}

#[test]
fn config_fingerprint_changes_only_when_thresholds_change() {
    let a = pipeline::analyze(&scope(), collected());
    let b = pipeline::analyze(&scope(), collected());
    assert_eq!(a.analyzer.config_fingerprint, b.analyzer.config_fingerprint);

    let mut manifest = conscience::ethics::manifest::Manifest::default();
    manifest.thresholds.ai_dependency_concern = 99.0;
    let mut s = scope();
    s.manifest = Some(manifest);
    let c = pipeline::analyze(&s, collected());
    assert_ne!(a.analyzer.config_fingerprint, c.analyzer.config_fingerprint);
    assert_eq!(c.analyzer.thresholds_source, "manifest");
}

#[test]
fn json_keeps_signals_at_top_level_and_round_trips() {
    let snap = pipeline::analyze(&scope(), collected());
    let json = serde_json::to_value(&snap).unwrap();

    // Existing consumers (the CI workflow) read data["signals"].
    assert!(json.get("signals").unwrap().is_array());
    assert!(json.get("scorecard").is_some());
    assert!(json.get("reflections").is_some());
    assert!(json.get("analysis").is_none(), "analysis must be flattened");
    assert!(json.get("snapshot_id").is_some());
    assert!(json.get("coverage").is_some());

    let back: Snapshot = serde_json::from_value(json).unwrap();
    assert_eq!(back.snapshot_id, snap.snapshot_id);
    assert_eq!(back.analysis.signals.len(), snap.analysis.signals.len());
}

#[test]
fn save_latest_and_find_by_prefix() {
    let root = std::env::temp_dir().join(format!("conscience-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let first = pipeline::analyze(&scope(), collected());
    let p1 = first.save(&root).unwrap();
    assert!(p1.starts_with(Snapshot::dir_for(&root)));

    // A second snapshot with a later id becomes "latest".
    let mut second = pipeline::analyze(&scope(), collected());
    second.snapshot_id = "20990101T000000Z-ffffff".into();
    second.save(&root).unwrap();

    let (path, latest) = Snapshot::latest(&root).unwrap().unwrap();
    assert_eq!(latest.snapshot_id, "20990101T000000Z-ffffff");
    assert!(path.ends_with("20990101T000000Z-ffffff.json"));

    // Prefix lookup, exact lookup, path lookup, and the two failure modes.
    let (_, found) = Snapshot::find(&root, "20990101").unwrap();
    assert_eq!(found.snapshot_id, second.snapshot_id);
    let (_, found) = Snapshot::find(&root, &first.snapshot_id).unwrap();
    assert_eq!(found.snapshot_id, first.snapshot_id);
    let (_, found) = Snapshot::find(&root, p1.to_str().unwrap()).unwrap();
    assert_eq!(found.snapshot_id, first.snapshot_id);
    assert!(
        Snapshot::find(&root, "nope")
            .unwrap_err()
            .to_string()
            .contains("No snapshot")
    );
    let ambiguous = Snapshot::find(&root, "2").unwrap_err().to_string();
    assert!(ambiguous.contains("matches 2 snapshots"), "{}", ambiguous);

    assert!(
        Snapshot::latest(&std::env::temp_dir().join("no-such-conscience-root"))
            .unwrap()
            .is_none()
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn summary_mentions_id_interval_coverage_and_counts() {
    let snap = pipeline::analyze(&scope(), collected());
    let s = snap.summary();
    assert!(s.contains(&snap.snapshot_id));
    assert!(s.contains("github collected"));
    assert!(s.contains("claude_code collected"));
    assert!(s.contains("warnings"));
    assert!(s.contains("thresholds"));
}
