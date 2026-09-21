//! Automation discovery and pruning, without touching the real crontab.

use chrono::{Duration, Utc};
use conscience::ai_tools::models::*;
use conscience::automation::*;
use conscience::ethics::signals;
use std::collections::HashMap;

fn run(cwd: &str, prompt: &str, days_ago: i64, error: Option<&str>, tokens: u64) -> AiSession {
    let start = Utc::now() - Duration::days(days_ago);
    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: format!("s-{}", days_ago),
        project_path: Some(cwd.into()),
        started_at: Some(start),
        ended_at: Some(start + Duration::minutes(1)),
        model: if error.is_some() {
            None
        } else {
            Some("claude-sonnet-4".into())
        },
        work_categories: vec![],
        turns: TurnCounts::default(),
        tokens: TokenUsage {
            input: 0,
            output: tokens,
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
        launch: Launch {
            entrypoint: Some("sdk-cli".into()),
            prompt_source: Some("sdk".into()),
            first_prompt: Some(prompt.into()),
            api_error: error.map(String::from),
        },
    }
}

#[test]
fn crontab_parsing_skips_comments_and_reads_schedules() {
    let text = "# marketing\n0 8 * * 1-5 /x/run.sh\n\nSHELL=/bin/bash\n@daily /y/other.sh arg\n5 8 * * * claude -p hi\n";
    let parsed = parse_crontab(text);
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].1, "0 8 * * 1-5");
    assert_eq!(parsed[0].2, "/x/run.sh");
    assert_eq!(parsed[1].1, "@daily");
    assert_eq!(parsed[1].2, "/y/other.sh arg");
    assert_eq!(parsed[2].2, "claude -p hi");
}

#[test]
fn cron_schedules_read_as_words() {
    assert_eq!(describe_cron("0 8 * * 1-5"), "weekdays at 08:00");
    assert_eq!(describe_cron("30 17 * * *"), "daily at 17:30");
    assert_eq!(describe_cron("0 9 1 * *"), "0 9 1 * *");
    assert_eq!(describe_cron("@daily"), "@daily");
}

#[test]
fn sdk_runs_group_into_signatures_with_cadence_and_observations() {
    let sessions = vec![
        run(
            "/work/mkt",
            "Read digest.md and execute it.",
            1,
            Some("authentication_failed"),
            0,
        ),
        run(
            "/work/mkt",
            "Read digest.md and execute it.",
            2,
            Some("authentication_failed"),
            0,
        ),
        run(
            "/work/mkt",
            "Read digest.md and execute it.",
            3,
            Some("authentication_failed"),
            0,
        ),
        run("/work/app", "Summarize the week", 7, None, 4000),
        run("/work/app", "Summarize the week", 14, None, 3000),
    ];
    let entries = discover_with(&sessions, "", &[], &[]);

    // No cron, launchd, or daemon inputs, so both are signatures.
    let mkt = entries
        .iter()
        .find(|e| e.label.contains("digest"))
        .expect("mkt entry");
    assert_eq!(mkt.source.kind(), "sdk runs");
    assert_eq!(mkt.runs, 3);
    assert_eq!(mkt.failures, 3);
    assert_eq!(mkt.last_success, None);
    assert_eq!(mkt.failure_reason.as_deref(), Some("authentication_failed"));
    assert_eq!(mkt.cadence, "every ~24h");
    assert!(
        mkt.observations
            .iter()
            .any(|o| o == "has never succeeded (3 runs)"),
        "{:?}",
        mkt.observations
    );

    let app = entries
        .iter()
        .find(|e| e.label == "Summarize the week")
        .unwrap();
    assert_eq!(app.failures, 0);
    assert!(app.last_success.is_some());
    assert_eq!(app.cadence, "every ~7d");
    assert_eq!(app.output_tokens, 7000);
    assert!(app.energy_wh > 0.0);

    // Ids are stable across discoveries.
    let again = discover_with(&sessions, "", &[], &[]);
    assert_eq!(
        again
            .iter()
            .find(|e| e.label.contains("digest"))
            .unwrap()
            .id,
        mkt.id
    );
    assert_eq!(mkt.id.len(), 6);
}

#[test]
fn interactive_sessions_are_not_automation() {
    let mut s = run("/work/app", "hello", 1, None, 10);
    s.launch = Launch::default();
    assert!(discover_with(&[s], "", &[], &[]).is_empty());
}

#[test]
fn remove_cron_line_matches_exactly_and_reports() {
    let text = "# a\n0 8 * * 1-5 /x/run.sh\n5 8 * * 1-5 /y/run.sh\n";
    let (out, removed) = remove_cron_line(text, "0 8 * * 1-5 /x/run.sh");
    assert!(removed);
    assert_eq!(out, "# a\n5 8 * * 1-5 /y/run.sh\n");
    let (same, removed) = remove_cron_line(&out, "0 8 * * 1-5 /x/run.sh");
    assert!(!removed);
    assert_eq!(same, out);
    // Removing the last remaining line leaves an empty crontab.
    let (empty, _) = remove_cron_line("5 8 * * 1-5 /y/run.sh\n", "5 8 * * 1-5 /y/run.sh");
    assert_eq!(empty, "\n");
}

#[test]
fn plan_names_what_would_change_and_signatures_cannot_be_pruned() {
    let home = std::env::temp_dir();
    let cron = Entry {
        id: "abc123".into(),
        source: Source::Cron {
            line: "0 8 * * 1-5 /x/run.sh".into(),
            schedule: "0 8 * * 1-5".into(),
            command: "/x/run.sh".into(),
        },
        label: "x".into(),
        cadence: String::new(),
        cwd: None,
        runs: 0,
        failures: 0,
        last_run: None,
        last_success: None,
        failure_reason: None,
        output_tokens: 0,
        energy_wh: 0.0,
        observations: vec![],
    };
    assert_eq!(
        plan(&cron, &home),
        PruneAction::RemoveCronLine {
            line: "0 8 * * 1-5 /x/run.sh".into()
        }
    );
    assert!(describe_action(&plan(&cron, &home)).contains("remove this line"));

    let sig = Entry {
        source: Source::SdkSignature {
            cwd: "/w".into(),
            prompt: "p".into(),
        },
        ..cron.clone()
    };
    assert!(matches!(
        plan(&sig, &home),
        PruneAction::NothingToRemove { .. }
    ));
    assert!(
        apply(&plan(&sig, &home)).is_err(),
        "nothing to apply must not pretend to succeed"
    );
}

#[test]
fn failing_automation_becomes_a_signal_with_count_only_evidence() {
    let sessions = vec![
        run(
            "/work/mkt",
            "Read digest.md",
            1,
            Some("authentication_failed"),
            0,
        ),
        run(
            "/work/mkt",
            "Read digest.md",
            2,
            Some("authentication_failed"),
            0,
        ),
        run(
            "/work/mkt",
            "Read digest.md",
            3,
            Some("authentication_failed"),
            0,
        ),
        run("/work/app", "Summarize", 1, None, 100),
    ];
    let summary = AiUsageSummary::from_sessions(AiTool::ClaudeCode, sessions);
    let sigs = signals::detect_ai_signals(&summary, None);
    let s = sigs
        .iter()
        .find(|s| s.id == "automation_failing_repeatedly")
        .expect("signal");
    assert!(
        s.detail
            .contains("1 recurring program-launched job(s) have run 3 times"),
        "{}",
        s.detail
    );
    assert!(s.detail.contains("authentication_failed"));
    assert_eq!(s.evidence, "1 job(s), 3 failed run(s)");
    assert!(
        !s.evidence.contains("digest"),
        "evidence carries no prompt text"
    );

    // Two failures is not yet a pattern.
    let few = AiUsageSummary::from_sessions(
        AiTool::ClaudeCode,
        vec![
            run(
                "/work/mkt",
                "Read digest.md",
                1,
                Some("authentication_failed"),
                0,
            ),
            run(
                "/work/mkt",
                "Read digest.md",
                2,
                Some("authentication_failed"),
                0,
            ),
        ],
    );
    assert!(
        signals::detect_ai_signals(&few, None)
            .iter()
            .all(|s| s.id != "automation_failing_repeatedly")
    );
}

#[test]
fn cron_entry_claims_the_sessions_its_script_launches() {
    let dir = std::env::temp_dir().join(format!("conscience-auto-cron-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("run-digest.sh");
    std::fs::write(
        &script,
        "#!/bin/bash\ncd /work/mkt\nclaude --print -p \"Read digest.md and execute it.\"\n",
    )
    .unwrap();
    let crontab = format!("# jobs\n0 8 * * 1-5 {}\n", script.display());
    let sessions = vec![
        run(
            "/work/mkt",
            "Read digest.md and execute it.",
            1,
            Some("authentication_failed"),
            0,
        ),
        run(
            "/work/mkt",
            "Read digest.md and execute it.",
            2,
            Some("authentication_failed"),
            0,
        ),
    ];

    let entries = discover_with(&sessions, &crontab, &[], &[]);
    assert_eq!(
        entries.len(),
        1,
        "the signature was attached to the cron entry: {:?}",
        entries
    );
    let e = &entries[0];
    assert_eq!(e.source.kind(), "cron");
    assert_eq!(e.cadence, "weekdays at 08:00");
    assert_eq!(e.runs, 2);
    assert_eq!(e.failures, 2);
    assert_eq!(e.label, "Read digest.md and execute it.");
    assert_eq!(e.cwd.as_deref(), Some("/work/mkt"));
    assert!(matches!(plan(e, &dir), PruneAction::RemoveCronLine { .. }));

    // A cron line that does not involve claude is not listed.
    let unrelated = "0 3 * * * /usr/bin/backup.sh\n";
    assert!(discover_with(&[], unrelated, &[], &[]).is_empty());

    std::fs::remove_dir_all(&dir).ok();
}
