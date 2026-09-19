//! Interval filtering rebuilds summary totals rather than just dropping
//! sessions, and reports undated sessions instead of silently including
//! or excluding them.

use chrono::{DateTime, Utc};
use conscience::ai_tools::models::*;
use conscience::interval::Interval;
use std::collections::HashMap;

fn session(id: &str, start: Option<&str>, output_tokens: u64, human_turns: u64) -> AiSession {
    let p = |s: &str| s.parse::<DateTime<Utc>>().unwrap();
    let mut tools_used = HashMap::new();
    tools_used.insert("Edit".to_string(), 1u64);
    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: id.into(),
        project_path: None,
        started_at: start.map(p),
        ended_at: start.map(|s| p(s) + chrono::Duration::hours(1)),
        model: Some("claude-fable-5-1".into()),
        work_categories: vec![],
        turns: TurnCounts {
            human: human_turns,
            assistant: human_turns,
            machine: 0,
            total: human_turns * 2,
        },
        tokens: TokenUsage {
            input: 10,
            output: output_tokens,
            cache_creation: 0,
            cache_read: 0,
        },
        tools_used,
        files_touched: vec![FileTouched {
            path: format!("/src/{}.rs", id),
            action: FileAction::Edit,
        }],
        bash_commands: vec![format!("echo {}", id)],
        agent_actions: vec![],
        git_branch: None,
        interactions: vec![],
        agent_dispatches: vec![],
        skill_invocations: vec![],
    }
}

#[test]
fn restrict_keeps_only_overlapping_sessions_and_rebuilds_every_total() {
    let all = AiUsageSummary::from_sessions(
        AiTool::ClaudeCode,
        vec![
            session("old", Some("2026-08-01T10:00:00Z"), 1000, 5),
            session("in1", Some("2026-09-03T10:00:00Z"), 100, 2),
            session("in2", Some("2026-09-05T10:00:00Z"), 200, 3),
            session("undated", None, 5000, 50),
        ],
    );
    assert_eq!(all.session_count, 4);
    assert_eq!(all.total_tokens.output, 6300);

    let iv = Interval::between(
        "2026-09-01T00:00:00Z".parse().unwrap(),
        "2026-09-08T00:00:00Z".parse().unwrap(),
    );
    let r = all.restrict(&iv);

    assert_eq!(r.session_count, 2);
    assert_eq!(r.undated_sessions, 1);
    assert_eq!(r.period_start, Some(iv.start));
    assert_eq!(r.period_end, Some(iv.end));

    // Every derived total reflects only the two in-range sessions.
    assert_eq!(r.total_tokens.output, 300);
    assert_eq!(r.total_tokens.input, 20);
    assert_eq!(r.total_turns.human, 5);
    assert_eq!(r.total_turns.total, 10);
    assert_eq!(r.models_used["claude-fable-5-1"], 2);
    assert_eq!(r.tools_used["Edit"], 2);
    assert_eq!(r.files_touched_count, 2);
    assert_eq!(r.unique_files_touched, 2);
    assert_eq!(r.all_bash_commands.len(), 2);
    assert!(
        r.all_bash_commands
            .iter()
            .all(|c| c.session_id.starts_with("in"))
    );

    // The original is untouched and carries no period.
    assert_eq!(all.session_count, 4);
    assert_eq!(all.period_start, None);
}

#[test]
fn restrict_with_nothing_in_range_is_empty_not_an_error() {
    let all = AiUsageSummary::from_sessions(
        AiTool::Codex,
        vec![session("old", Some("2026-01-01T10:00:00Z"), 10, 1)],
    );
    let r = all.restrict(&Interval::last_days(7));
    assert_eq!(r.session_count, 0);
    assert_eq!(r.total_tokens.output, 0);
    assert_eq!(r.undated_sessions, 0);
    assert_eq!(r.tool, AiTool::Codex);
}

#[test]
fn summary_deserializes_without_the_new_period_fields() {
    // Older JSON (e.g. a saved ingest) must still load.
    let json = r#"{"tool":"claude_code","session_count":0,"total_tokens":{"input":0,"output":0,"cache_creation":0,"cache_read":0},"total_turns":{"human":0,"assistant":0,"total":0},"models_used":{},"tools_used":{},"files_touched_count":0,"unique_files_touched":0,"all_bash_commands":[],"agent_actions_summary":{"total_actions":0,"messages_sent":0,"emails_sent":0,"meetings_scheduled":0,"documents_created":0,"approvals_requested":0,"approvals_granted":0,"approvals_denied":0,"actions_without_approval":0,"channels_used":[]},"sessions":[]}"#;
    let s: AiUsageSummary = serde_json::from_str(json).unwrap();
    assert_eq!(s.period_start, None);
    assert_eq!(s.undated_sessions, 0);
}
