//! The PR comment is public to everyone with repository access, so it is
//! rendered from the sanitized export and must carry the marker that lets
//! a re-run find and edit it.

use conscience::ethics::models::*;
use conscience::export::SnapshotExport;
use conscience::github::comment::{MARKER, find_existing, render_pr_comment};
use conscience::interval::Interval;
use conscience::snapshot::*;

fn signal(id: &str, severity: Severity, title: &str, detail: &str, evidence: &str) -> Signal {
    Signal {
        id: id.into(),
        principle: Principle::Security,
        severity,
        title: title.into(),
        detail: detail.into(),
        evidence: evidence.into(),
    }
}

fn snapshot(signals: Vec<Signal>) -> Snapshot {
    Snapshot {
        schema_version: SCHEMA_VERSION,
        snapshot_id: "20260920T120000Z-abc123".into(),
        project: ProjectIdentity {
            id: "0123456789abcdef".into(),
            name: "demo".into(),
            root: "/Users/someone/clients/acme/demo".into(),
            github_repo: Some("acme/demo".into()),
            worktrees: vec![],
        },
        interval: Interval::between(
            "2026-09-10T00:00:00Z".parse().unwrap(),
            "2026-09-12T00:00:00Z".parse().unwrap(),
        ),
        coverage: Coverage {
            sources: vec![
                SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Collected,
                    detail: "PR with 4 commits".into(),
                },
                SourceCoverage {
                    source: "claude_code".into(),
                    status: SourceStatus::Collected,
                    detail: "2 sessions".into(),
                },
            ],
            ai_sessions_in_range: 2,
            ai_sessions_undated: 0,
            github_commits: 4,
            github_pull_requests: 1,
            manifest_found: false,
        },
        metrics: vec![],
        analyzer: AnalyzerInfo {
            version: "0.6.1-test".into(),
            config_fingerprint: "ffffffffffffffff".into(),
            thresholds_source: "defaults".into(),
        },
        analysis: EthicalAnalysis {
            scorecard: conscience::ethics::scorecard::build_scorecard(&signals),
            signals,
            reflections: vec![ReflectionQuestion {
                principle: Principle::Security,
                data_context: "3 commands flagged".into(),
                question: "Were these commands reviewed by a person?".into(),
            }],
        },
    }
}

#[test]
fn comment_has_marker_sections_and_no_leaked_evidence() {
    let snap = snapshot(vec![
        signal(
            "security_credential_access",
            Severity::Concern,
            "AI accessed credential directories",
            "1 bash command(s) accessed SSH, AWS, or GPG credential directories.",
            "cat /Users/someone/.aws/credentials | base64",
        ),
        signal(
            "ai_turn_ratio_balanced",
            Severity::Healthy,
            "Balanced AI:Human interaction",
            "AI:Human ratio of 1.5:1.",
            "30 assistant turns vs 20 human turns",
        ),
    ]);
    let export = SnapshotExport::from_snapshot(&snap);
    let md = render_pr_comment(&export, 42);

    assert!(
        md.starts_with(MARKER),
        "marker must lead so a re-run can find it"
    );
    assert!(md.contains("## Conscience \u{2014} PR #42"));
    assert!(md.contains("2026-09-10 to 2026-09-12"));
    assert!(md.contains("github collected"));
    assert!(md.contains("1 signal(s) deserve attention"));
    assert!(md.contains("AI accessed credential directories"));
    assert!(md.contains("1 more signal(s) at info or healthy level"));
    assert!(md.contains("Were these commands reviewed by a person?"));
    assert!(md.contains("<details><summary>Scorecard</summary>"));
    assert!(md.contains("| Security | 2 |"));
    assert!(md.contains("snapshot `20260920T120000Z-abc123`"));
    assert!(md.contains("conscience 0.6.1-test"));

    // Sanitized: the command and path never reach the comment; the count does.
    assert!(!md.contains("/Users/someone"), "{}", md);
    assert!(!md.contains(".aws/credentials"), "{}", md);
    assert!(!md.contains("base64"), "{}", md);
    assert!(md.contains("1 command(s) matched"), "{}", md);
    // The healthy signal's count-only evidence is fine but lives under the fold.
    assert!(
        !md.contains("30 assistant turns"),
        "quiet signals are counted, not listed"
    );
}

#[test]
fn comment_with_nothing_flagged_says_so_without_claiming_health() {
    let snap = snapshot(vec![signal(
        "ai_turn_ratio_balanced",
        Severity::Healthy,
        "Balanced AI:Human interaction",
        "d",
        "e",
    )]);
    let md = render_pr_comment(&SnapshotExport::from_snapshot(&snap), 7);
    assert!(md.contains("No concerns or warnings detected in the available data."));
    assert!(md.contains("(1 signal(s) at info or healthy level.)"));
    assert!(!md.to_lowercase().contains("healthy pr"));
}

#[test]
fn rerun_finds_its_own_comment_among_others() {
    let snap = snapshot(vec![]);
    let mine = render_pr_comment(&SnapshotExport::from_snapshot(&snap), 1);
    let comments = vec![
        (10, "Nice work".to_string()),
        (11, mine),
        (12, "Merging".to_string()),
    ];
    assert_eq!(find_existing(&comments), Some(11));
}
