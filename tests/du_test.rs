//! du measures only what conscience wrote, keeps the logs it reads apart,
//! and tidy never removes what history compares.

use chrono::{Duration, Utc};
use conscience::du::*;
use conscience::ethics::models::EthicalAnalysis;
use conscience::interval::Interval;
use conscience::snapshot::*;
use std::fs;
use std::path::{Path, PathBuf};

fn temp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("conscience-du-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn snapshot(id: &str, days_ago: i64, interval_days: i64, pr: bool) -> Snapshot {
    let collected = Utc::now() - Duration::days(days_ago);
    let mut interval = Interval::between(collected - Duration::days(interval_days), collected);
    interval.collected_at = collected;
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
                    "PR with 1 commits".into()
                } else {
                    "3 commits".into()
                },
            }],
            ..Default::default()
        },
        metrics: vec![],
        analyzer: AnalyzerInfo {
            version: "t".into(),
            config_fingerprint: "f".into(),
            thresholds_source: "defaults".into(),
        },
        analysis: EthicalAnalysis {
            signals: vec![],
            scorecard: vec![],
            reflections: vec![],
        },
    }
}

/// Ids must sort by time for Snapshot::list; build them from days_ago.
fn id_for(days_ago: i64, tag: &str) -> String {
    let t = Utc::now() - Duration::days(days_ago);
    format!("{}-{}", t.format("%Y%m%dT%H%M%SZ"), tag)
}

#[test]
fn measure_separates_owned_from_read_and_sizes_each() {
    let home = temp("home");
    let project = temp("project");
    // Owned: two snapshots and one reflection in the project.
    snapshot(&id_for(1, "aaaaaa"), 1, 7, false)
        .save(&project)
        .unwrap();
    snapshot(&id_for(2, "bbbbbb"), 2, 7, false)
        .save(&project)
        .unwrap();
    fs::create_dir_all(project.join(".conscience/reflections")).unwrap();
    fs::write(project.join(".conscience/reflections/r.json"), "{}").unwrap();
    // Owned: prune backups under home.
    fs::create_dir_all(home.join(".claude/pruned")).unwrap();
    fs::write(home.join(".claude/pruned/crontab.bak"), "0 8 * * * x\n").unwrap();
    // Read: fake tool logs under home.
    fs::create_dir_all(home.join(".claude/projects/-work-demo")).unwrap();
    fs::write(
        home.join(".claude/projects/-work-demo/s.jsonl"),
        "x".repeat(5000),
    )
    .unwrap();
    fs::create_dir_all(home.join(".codex/sessions/2026")).unwrap();
    fs::write(home.join(".codex/sessions/2026/r.jsonl"), "y".repeat(1000)).unwrap();

    let u = measure(&[project.clone()], &home);

    let kinds: Vec<&str> = u.owned.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"snapshots"), "{:?}", kinds);
    assert!(kinds.contains(&"reflections"), "{:?}", kinds);
    assert!(kinds.contains(&"prune backups"), "{:?}", kinds);
    let snaps = u.owned.iter().find(|e| e.kind == "snapshots").unwrap();
    assert_eq!(snaps.files, 2);
    assert!(snaps.bytes > 0);
    assert_eq!(u.owned_bytes, u.owned.iter().map(|e| e.bytes).sum::<u64>());

    assert_eq!(u.read.len(), 2);
    assert_eq!(u.read_bytes, 6000);
    assert!(
        u.read.iter().all(|r| !r.path.contains(".conscience")),
        "logs are never owned"
    );

    let g = u
        .growth
        .iter()
        .find(|g| g.project == project.to_string_lossy())
        .unwrap();
    assert_eq!(g.snapshots_last_30_days, 2);
    assert_eq!(
        g.bytes_per_year_at_this_rate,
        g.bytes_last_30_days * 365 / 30
    );

    let text = render(&u);
    assert!(text.contains("Read, never written or deleted"));
    assert!(text.contains("more is read than written"));

    fs::remove_dir_all(&home).ok();
    fs::remove_dir_all(&project).ok();
}

#[test]
fn tidy_removes_old_snapshots_but_keeps_two_per_group_and_reflections() {
    let project = temp("tidy");
    // 7-day group: four old ones and one recent.
    for (ago, tag) in [
        (200, "a1"),
        (150, "a2"),
        (120, "a3"),
        (100, "a4"),
        (5, "a5"),
    ] {
        snapshot(&id_for(ago, tag), ago, 7, false)
            .save(&project)
            .unwrap();
    }
    // 30-day group: two old ones only; both must survive.
    for (ago, tag) in [(300, "b1"), (250, "b2")] {
        snapshot(&id_for(ago, tag), ago, 30, false)
            .save(&project)
            .unwrap();
    }
    // PR snapshots: three old; two survive.
    for (ago, tag) in [(400, "c1"), (350, "c2"), (320, "c3")] {
        snapshot(&id_for(ago, tag), ago, 1, true)
            .save(&project)
            .unwrap();
    }
    fs::create_dir_all(project.join(".conscience/reflections")).unwrap();
    fs::write(project.join(".conscience/reflections/r.json"), "{}").unwrap();

    let plan = plan_tidy(&project, 90);
    let removed: Vec<&str> = plan
        .remove
        .iter()
        .map(|(p, _)| Path::new(p).file_stem().unwrap().to_str().unwrap())
        .collect();
    // 7d: newest two (a5, a4) protected; a3, a2, a1 are old -> removed.
    assert!(removed.iter().any(|r| r.ends_with("a1")), "{:?}", removed);
    assert!(removed.iter().any(|r| r.ends_with("a2")), "{:?}", removed);
    assert!(removed.iter().any(|r| r.ends_with("a3")), "{:?}", removed);
    assert!(!removed.iter().any(|r| r.ends_with("a4")), "{:?}", removed);
    assert!(!removed.iter().any(|r| r.ends_with("a5")), "{:?}", removed);
    // 30d: both protected despite age.
    assert!(
        !removed
            .iter()
            .any(|r| r.ends_with("b1") || r.ends_with("b2")),
        "{:?}",
        removed
    );
    // pr: oldest one goes, two stay.
    assert!(removed.iter().any(|r| r.ends_with("c1")), "{:?}", removed);
    assert!(
        !removed
            .iter()
            .any(|r| r.ends_with("c2") || r.ends_with("c3")),
        "{:?}",
        removed
    );
    assert_eq!(plan.remove.len(), 4);
    assert_eq!(plan.kept, 6);
    assert!(plan.bytes_freed > 0);

    let text = render_tidy(&plan, 90);
    assert!(text.contains("4 to remove"));
    assert!(text.contains("Reflections and AI tool logs are not touched"));

    assert_eq!(apply_tidy(&plan).unwrap(), 4);
    assert_eq!(Snapshot::list(&project).len(), 6);
    assert!(project.join(".conscience/reflections/r.json").exists());

    // Nothing left to tidy.
    assert!(plan_tidy(&project, 90).remove.is_empty());

    fs::remove_dir_all(&project).ok();
}

#[test]
fn a_project_with_nothing_written_measures_empty() {
    let home = temp("empty-home");
    let project = temp("empty-project");
    let u = measure(&[project.clone()], &home);
    assert!(u.owned.is_empty());
    assert_eq!(u.owned_bytes, 0);
    assert!(render(&u).contains("nothing written yet"));
    fs::remove_dir_all(&home).ok();
    fs::remove_dir_all(&project).ok();
}
