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
fn human_turns_count_only_genuine_prompts() {
    let session = parse_fixture("basic_session.jsonl");
    assert_eq!(session.turns.human, 2, "u1 and u5 only");
    assert_eq!(session.turns.machine, 2, "tool_result u2 and caveat u4");
    assert_eq!(session.turns.assistant, 3);
    assert_eq!(session.turns.total, 5, "human + assistant");
}
