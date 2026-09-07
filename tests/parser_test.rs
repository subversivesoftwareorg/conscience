use conscience::ai_tools::claude_code::ClaudeCodeParser;
use std::path::Path;

fn parse_fixture(name: &str) -> conscience::ai_tools::models::AiSession {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    ClaudeCodeParser::new()
        .parse_session_file("test-session", &path)
        .expect("fixture parses")
}

#[test]
fn interactions_capture_prompt_and_response_end() {
    let session = parse_fixture("basic_session.jsonl");
    assert_eq!(session.interactions.len(), 2);

    let first = &session.interactions[0];
    assert_eq!(first.uuid.as_deref(), Some("u1"));
    assert_eq!(first.human_at.to_rfc3339(), "2026-09-01T16:40:34.873+00:00");
    assert_eq!(
        first.ai_until.unwrap().to_rfc3339(),
        "2026-09-01T16:40:55+00:00"
    );

    let second = &session.interactions[1];
    assert_eq!(second.uuid.as_deref(), Some("u5"));
    assert_eq!(
        second.ai_until.unwrap().to_rfc3339(),
        "2026-09-01T16:41:20+00:00"
    );
}

#[test]
fn human_turns_count_only_genuine_prompts() {
    let session = parse_fixture("basic_session.jsonl");
    assert_eq!(session.turns.human, 2, "u1 and u5 only");
    assert_eq!(session.turns.machine, 2, "tool_result u2 and caveat u4");
    assert_eq!(session.turns.assistant, 3);
    assert_eq!(session.turns.total, 5, "human + assistant");
}
