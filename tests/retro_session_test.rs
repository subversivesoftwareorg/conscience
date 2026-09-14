use conscience::ai_tools::models::*;
use conscience::analysis::session_retro::*;
use std::collections::HashMap;

fn make_session_full(
    id: &str,
    project: &str,
    output: u64,
    input: u64,
    cache_create: u64,
    cache_read: u64,
    tools: Vec<(&str, u64)>,
    human: u64,
    assistant: u64,
    hours: f64,
    agents: Vec<AgentDispatch>,
    skills: Vec<SkillInvocation>,
) -> AiSession {
    let now = chrono::Utc::now();
    let mut tools_used = HashMap::new();
    for (name, count) in tools {
        tools_used.insert(name.to_string(), count);
    }
    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: id.to_string(),
        project_path: Some(project.to_string()),
        started_at: Some(now - chrono::Duration::minutes((hours * 60.0) as i64)),
        ended_at: Some(now),
        model: Some("claude-fable-5".to_string()),
        work_categories: vec![WorkCategory::Code],
        turns: TurnCounts { human, assistant, machine: 0, total: human + assistant },
        tokens: TokenUsage { input, output, cache_creation: cache_create, cache_read },
        tools_used,
        files_touched: vec![],
        bash_commands: vec![],
        agent_actions: Vec::new(),
        git_branch: Some("main".to_string()),
        interactions: Vec::new(),
        agent_dispatches: agents,
        skill_invocations: skills,
    }
}

fn simple_session(id: &str, project: &str, output: u64, input: u64) -> AiSession {
    make_session_full(id, project, output, input, 0, 0, vec![], 5, 10, 1.0, vec![], vec![])
}

#[test]
fn retro_ranks_sessions_by_total_tokens() {
    let sessions = vec![
        simple_session("small", "/p/a", 10_000, 50_000),
        simple_session("big", "/p/b", 500_000, 2_000_000),
        simple_session("medium", "/p/c", 100_000, 300_000),
    ];
    let retro = analyze_token_retro(&sessions);
    assert_eq!(retro.per_session[0].session_id, "big");
    assert_eq!(retro.per_session[2].session_id, "small");
    assert_eq!(retro.total_tokens, 2_960_000);
}

#[test]
fn retro_diagnoses_input_dominated() {
    let sessions = vec![
        make_session_full("s1", "/p/a", 50_000, 500_000, 0, 0, vec![], 5, 10, 1.0, vec![], vec![]),
    ];
    let retro = analyze_token_retro(&sessions);
    assert!(retro.diagnoses.iter().any(|d| d.to_lowercase().contains("input")),
        "should diagnose input dominance, got: {:?}", retro.diagnoses);
}

#[test]
fn retro_diagnoses_low_cache_efficiency() {
    let sessions = vec![
        make_session_full("s1", "/p/a", 50_000, 200_000, 180_000, 20_000, vec![], 5, 10, 1.0, vec![], vec![]),
    ];
    let retro = analyze_token_retro(&sessions);
    assert!(retro.diagnoses.iter().any(|d| d.to_lowercase().contains("cache")),
        "should diagnose low cache efficiency, got: {:?}", retro.diagnoses);
}

#[test]
fn retro_diagnoses_heavy_agent_usage() {
    let agents = vec![
        AgentDispatch { description: "Implement Task 1".to_string(), agent_type: Some("general-purpose".to_string()) },
        AgentDispatch { description: "Review Task 1".to_string(), agent_type: Some("general-purpose".to_string()) },
        AgentDispatch { description: "Implement Task 2".to_string(), agent_type: Some("general-purpose".to_string()) },
        AgentDispatch { description: "Review Task 2".to_string(), agent_type: Some("general-purpose".to_string()) },
        AgentDispatch { description: "Implement Task 3".to_string(), agent_type: Some("general-purpose".to_string()) },
        AgentDispatch { description: "Review Task 3".to_string(), agent_type: Some("general-purpose".to_string()) },
    ];
    let sessions = vec![
        make_session_full("s1", "/p/a", 200_000, 800_000, 0, 0, vec![], 10, 50, 2.0, agents, vec![]),
    ];
    let retro = analyze_token_retro(&sessions);
    assert!(retro.diagnoses.iter().any(|d| d.to_lowercase().contains("agent")),
        "should diagnose heavy agent usage, got: {:?}", retro.diagnoses);
    assert_eq!(retro.per_session[0].agent_count, 6);
}

#[test]
fn retro_reports_skill_usage() {
    let skills = vec![
        SkillInvocation { skill: "superpowers:brainstorming".to_string() },
        SkillInvocation { skill: "superpowers:writing-plans".to_string() },
        SkillInvocation { skill: "superpowers:subagent-driven-development".to_string() },
    ];
    let sessions = vec![
        make_session_full("s1", "/p/a", 100_000, 400_000, 0, 0, vec![], 5, 15, 1.0, vec![], skills),
    ];
    let retro = analyze_token_retro(&sessions);
    assert_eq!(retro.per_session[0].skill_count, 3);
    assert!(retro.per_session[0].skills.contains(&"superpowers:subagent-driven-development".to_string()));
}

#[test]
fn retro_handles_empty_sessions() {
    let retro = analyze_token_retro(&[]);
    assert_eq!(retro.total_tokens, 0);
    assert!(retro.per_session.is_empty());
    assert!(retro.diagnoses.is_empty());
}

#[test]
fn retro_includes_energy_estimate() {
    let sessions = vec![simple_session("s1", "/p/a", 100_000, 200_000)];
    let retro = analyze_token_retro(&sessions);
    assert!(retro.total_energy_wh > 0.0);
}
