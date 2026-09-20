//! History compares like with like and answers the questions it exists
//! for, from synthetic snapshots and no network.

use chrono::{DateTime, Duration, Utc};
use conscience::ethics::models::*;
use conscience::history::*;
use conscience::interval::Interval;
use conscience::snapshot::*;

fn snap(
    id: &str,
    collected_days_ago: i64,
    interval_days: i64,
    pr: bool,
    metrics: Vec<Metric>,
    signals: Vec<(&str, Severity)>,
    fingerprint: &str,
) -> Snapshot {
    let collected: DateTime<Utc> = Utc::now() - Duration::days(collected_days_ago);
    let mut interval = Interval::between(collected - Duration::days(interval_days), collected);
    interval.collected_at = collected;
    let signals: Vec<Signal> = signals
        .into_iter()
        .map(|(sid, sev)| Signal {
            id: sid.into(),
            principle: if sid.starts_with("security") {
                Principle::Security
            } else {
                Principle::HumanAgency
            },
            severity: sev,
            title: sid.replace('_', " "),
            detail: "d".into(),
            evidence: "e".into(),
        })
        .collect();
    Snapshot {
        schema_version: SCHEMA_VERSION,
        snapshot_id: id.into(),
        project: ProjectIdentity {
            id: "p".into(),
            name: "demo".into(),
            root: "/work/demo".into(),
            github_repo: None,
            worktrees: vec![],
        },
        interval,
        coverage: Coverage {
            sources: vec![SourceCoverage {
                source: "github".into(),
                status: SourceStatus::Collected,
                detail: if pr {
                    "PR with 3 commits".into()
                } else {
                    "9 commits, 4 PRs".into()
                },
            }],
            ..Default::default()
        },
        metrics,
        analyzer: AnalyzerInfo {
            version: "0.6.1".into(),
            config_fingerprint: fingerprint.into(),
            thresholds_source: "defaults".into(),
        },
        analysis: EthicalAnalysis {
            scorecard: conscience::ethics::scorecard::build_scorecard(&signals),
            signals,
            reflections: vec![],
        },
    }
}

fn m(key: &str, v: f64) -> Metric {
    Metric::observed(key, v, "x")
}

fn week(
    id: &str,
    ago: i64,
    ratio: f64,
    comments: f64,
    wh: f64,
    tokens: f64,
    sigs: Vec<(&str, Severity)>,
    fp: &str,
) -> Snapshot {
    snap(
        id,
        ago,
        7,
        false,
        vec![
            m("ai.sessions", 4.0),
            Metric::ratio("ai.turn_ratio", ratio, "assistant_turns", "human_turns"),
            m("ai.output_tokens", tokens),
            Metric::estimated("energy.wh", wh, "Wh", wh * 0.5, wh * 1.5),
            m("github.prs_merged", 3.0),
            Metric::ratio(
                "github.review_comments_per_merged_pr",
                comments,
                "comments",
                "merged_prs",
            ),
        ],
        sigs,
        fp,
    )
}

#[test]
fn groups_by_interval_and_ignores_pr_snapshots_for_comparison() {
    let entries: Vec<Entry> = vec![
        week("w-new", 0, 8.0, 2.0, 2000.0, 100_000.0, vec![], "cfg"),
        snap("pr-1", 1, 2, true, vec![], vec![], "cfg"),
        snap(
            "m-only",
            2,
            30,
            false,
            vec![m("ai.sessions", 10.0)],
            vec![],
            "cfg",
        ),
        week("w-old", 7, 4.0, 4.0, 1000.0, 100_000.0, vec![], "cfg"),
    ]
    .iter()
    .map(Entry::from_snapshot)
    .collect();

    let h = build(entries);
    assert_eq!(h.entries.len(), 4, "everything is listed");
    assert!(h.entries.iter().any(|e| e.is_pr));
    assert_eq!(
        h.comparisons.len(),
        1,
        "only the 7-day group has two snapshots"
    );
    let c = &h.comparisons[0];
    assert_eq!(c.interval_days, 7);
    assert_eq!(c.from_id, "w-old");
    assert_eq!(c.to_id, "w-new");
    assert_eq!(h.singletons, vec![30]);
}

#[test]
fn comparison_math_readings_and_signal_diffs() {
    let old = Entry::from_snapshot(&week(
        "old",
        7,
        4.0,
        4.0,
        1000.0,
        100_000.0,
        vec![
            ("security_credential_access", Severity::Concern),
            ("ai_turn_ratio_moderate", Severity::Info),
        ],
        "cfg",
    ));
    let new = Entry::from_snapshot(&week(
        "new",
        0,
        8.0,
        2.0,
        2400.0,
        120_000.0,
        vec![("ai_turn_ratio_high", Severity::Concern)],
        "cfg",
    ));
    let c = compare(&old, &new);

    let ratio = c.metrics.iter().find(|x| x.key == "ai.turn_ratio").unwrap();
    assert_eq!(ratio.from, 4.0);
    assert_eq!(ratio.to, 8.0);
    assert!((ratio.pct.unwrap() - 100.0).abs() < 1e-9);
    assert_eq!(ratio.kind, MetricKind::Observation);
    let wh = c.metrics.iter().find(|x| x.key == "energy.wh").unwrap();
    assert_eq!(wh.kind, MetricKind::Estimate);

    assert_eq!(c.appeared.len(), 1);
    assert_eq!(c.appeared[0].0, "ai_turn_ratio_high");
    assert_eq!(c.resolved.len(), 2);
    assert_eq!(c.concerns, (1, 1));
    assert!(!c.thresholds_changed);

    let text = c.readings.join("\n");
    assert!(
        text.contains("turn ratio is rising (4.0 to 8.0)"),
        "{}",
        text
    );
    assert!(
        text.contains("Review comments per merged PR are falling (4.0 to 2.0)"),
        "{}",
        text
    );
    // 10 Wh/1K -> 20 Wh/1K
    assert!(
        text.contains("Energy per 1K output tokens is rising"),
        "{}",
        text
    );
    assert!(
        text.contains("Security signals are trending down (1 to 0)"),
        "{}",
        text
    );
    assert!(!text.contains("Thresholds changed"));
}

#[test]
fn threshold_change_is_called_out_and_zero_baselines_do_not_divide() {
    let old = Entry::from_snapshot(&week("old", 7, 0.0, 0.0, 0.0, 0.0, vec![], "cfg-a"));
    let new = Entry::from_snapshot(&week("new", 0, 5.0, 1.0, 100.0, 1000.0, vec![], "cfg-b"));
    let c = compare(&old, &new);
    assert!(c.thresholds_changed);
    for key in [
        "ai.turn_ratio",
        "ai.output_tokens",
        "energy.wh",
        "github.review_comments_per_merged_pr",
    ] {
        let x = c.metrics.iter().find(|x| x.key == key).unwrap();
        assert!(
            x.pct.is_none(),
            "{} had a zero baseline, got {:?}",
            key,
            x.pct
        );
    }
    let text = c.readings.join("\n");
    assert!(text.contains("Thresholds changed"), "{}", text);
    assert!(text.contains("not enough data to compare"), "{}", text);
}

#[test]
fn render_lists_entries_and_explains_singletons() {
    let h = build(vec![Entry::from_snapshot(&week(
        "only",
        0,
        3.0,
        1.0,
        500.0,
        50_000.0,
        vec![],
        "cfg",
    ))]);
    let out = render(&h);
    assert!(out.contains("Snapshots (1, newest first)"));
    assert!(out.contains("only"));
    assert!(out.contains("Nothing to compare yet"));
    assert!(out.contains("nothing here is summed"));

    let empty = render(&build(vec![]));
    assert!(empty.contains("No snapshots in the window"));
}

#[test]
fn load_reads_saved_snapshots_within_the_window() {
    let root = std::env::temp_dir().join(format!("conscience-hist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    week("recent", 1, 1.0, 1.0, 1.0, 1.0, vec![], "c")
        .save(&root)
        .unwrap();
    week("ancient", 400, 1.0, 1.0, 1.0, 1.0, vec![], "c")
        .save(&root)
        .unwrap();

    let entries = load(&root, Utc::now() - Duration::days(90));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].snapshot_id, "recent");

    std::fs::remove_dir_all(&root).ok();
}
