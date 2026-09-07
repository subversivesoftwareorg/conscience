use conscience::ai_tools::models::*;
use conscience::analysis::authorship;
use conscience::ethics::models::*;
use conscience::github::models::*;
use chrono::{Duration, Utc};
use std::collections::HashMap;

fn make_commit_at(author: &str, time: chrono::DateTime<chrono::Utc>) -> CommitSummary {
    CommitSummary {
        sha: format!("{:040x}", time.timestamp()),
        author: author.to_string(),
        message: "test commit".to_string(),
        date: time,
        additions: None,
        deletions: None,
    }
}

fn make_session_at(
    id: &str,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    writes: u64,
) -> AiSession {
    let mut tools_used = HashMap::new();
    if writes > 0 { tools_used.insert("Write".to_string(), writes); }

    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: id.to_string(),
        project_path: None,
        started_at: Some(start),
        ended_at: Some(end),
        model: Some("claude-opus-4-6".to_string()),
        work_categories: vec![WorkCategory::Code],
        turns: TurnCounts { human: 10, assistant: 15, machine: 0, total: 25 },
        tokens: TokenUsage { input: 100, output: 50_000, cache_creation: 0, cache_read: 0 },
        tools_used,
        files_touched: Vec::new(),
        bash_commands: Vec::new(),
        agent_actions: Vec::new(),
        git_branch: Some("main".to_string()),
        interactions: Vec::new(),
    }
}

fn make_summary(sessions: Vec<AiSession>) -> AiUsageSummary {
    AiUsageSummary {
        tool: AiTool::ClaudeCode,
        session_count: sessions.len() as u64,
        total_tokens: TokenUsage::default(),
        total_turns: TurnCounts::default(),
        models_used: HashMap::new(),
        tools_used: HashMap::new(),
        files_touched_count: 0,
        unique_files_touched: 0,
        all_bash_commands: Vec::new(),
        agent_actions_summary: AgentActionsSummary::default(),
        sessions,
    }
}

#[test]
fn test_commit_during_session_is_correlated() {
    let now = Utc::now();
    let session_start = now - Duration::hours(2);
    let session_end = now - Duration::hours(1);
    let commit_time = now - Duration::minutes(90); // during session

    let repo = RepoSummary {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        period_start: now - Duration::days(7),
        period_end: now,
        commits: vec![make_commit_at("alice", commit_time)],
        pull_requests: Vec::new(),
    };

    let ai = make_summary(vec![
        make_session_at("session1", session_start, session_end, 5),
    ]);

    let result = authorship::analyze_authorship(&repo, &ai);
    assert_eq!(result.total_ai_correlated, 1);
    assert_eq!(result.contributors[0].ai_correlation_pct, 100.0);
}

#[test]
fn test_commit_after_session_within_window() {
    let now = Utc::now();
    let session_start = now - Duration::hours(3);
    let session_end = now - Duration::hours(2);
    let commit_time = now - Duration::minutes(105); // 15 min after session end

    let repo = RepoSummary {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        period_start: now - Duration::days(7),
        period_end: now,
        commits: vec![make_commit_at("alice", commit_time)],
        pull_requests: Vec::new(),
    };

    let ai = make_summary(vec![
        make_session_at("session1", session_start, session_end, 5),
    ]);

    let result = authorship::analyze_authorship(&repo, &ai);
    assert_eq!(result.total_ai_correlated, 1);
}

#[test]
fn test_commit_outside_window_not_correlated() {
    let now = Utc::now();
    let session_start = now - Duration::hours(5);
    let session_end = now - Duration::hours(4);
    let commit_time = now - Duration::hours(1); // 3 hours after session

    let repo = RepoSummary {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        period_start: now - Duration::days(7),
        period_end: now,
        commits: vec![make_commit_at("alice", commit_time)],
        pull_requests: Vec::new(),
    };

    let ai = make_summary(vec![
        make_session_at("session1", session_start, session_end, 5),
    ]);

    let result = authorship::analyze_authorship(&repo, &ai);
    assert_eq!(result.total_ai_correlated, 0);
}

#[test]
fn test_session_without_writes_not_correlated() {
    let now = Utc::now();
    let session_start = now - Duration::hours(2);
    let session_end = now - Duration::hours(1);
    let commit_time = now - Duration::minutes(90);

    let repo = RepoSummary {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        period_start: now - Duration::days(7),
        period_end: now,
        commits: vec![make_commit_at("alice", commit_time)],
        pull_requests: Vec::new(),
    };

    let ai = make_summary(vec![
        make_session_at("session1", session_start, session_end, 0), // no writes
    ]);

    let result = authorship::analyze_authorship(&repo, &ai);
    assert_eq!(result.total_ai_correlated, 0);
}

#[test]
fn test_high_correlation_generates_warning() {
    let now = Utc::now();
    let session_start = now - Duration::hours(3);
    let session_end = now;

    let commits: Vec<CommitSummary> = (0..10)
        .map(|i| make_commit_at("alice", now - Duration::minutes(i * 10)))
        .collect();

    let repo = RepoSummary {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        period_start: now - Duration::days(7),
        period_end: now,
        commits,
        pull_requests: Vec::new(),
    };

    let ai = make_summary(vec![
        make_session_at("session1", session_start, session_end, 20),
    ]);

    let result = authorship::analyze_authorship(&repo, &ai);
    assert!(result.overall_ai_correlation_pct > 80.0);
    assert!(result.signals.iter().any(|s| s.severity == Severity::Warning));
}

#[test]
fn test_multiple_contributors() {
    let now = Utc::now();
    let session_start = now - Duration::hours(2);
    let session_end = now - Duration::hours(1);

    let commits = vec![
        make_commit_at("alice", now - Duration::minutes(90)), // during session
        make_commit_at("alice", now - Duration::minutes(85)), // during session
        make_commit_at("bob", now - Duration::hours(5)),      // way outside
        make_commit_at("bob", now - Duration::hours(6)),      // way outside
    ];

    let repo = RepoSummary {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        period_start: now - Duration::days(7),
        period_end: now,
        commits,
        pull_requests: Vec::new(),
    };

    let ai = make_summary(vec![
        make_session_at("session1", session_start, session_end, 10),
    ]);

    let result = authorship::analyze_authorship(&repo, &ai);
    assert_eq!(result.total_ai_correlated, 2);

    let alice = result.contributors.iter().find(|c| c.author == "alice").unwrap();
    assert_eq!(alice.ai_correlation_pct, 100.0);

    let bob = result.contributors.iter().find(|c| c.author == "bob").unwrap();
    assert_eq!(bob.ai_correlation_pct, 0.0);
}
