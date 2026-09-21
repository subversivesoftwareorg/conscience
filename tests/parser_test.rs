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

#[test]
fn synthetic_records_never_label_a_session() {
    // Claude Code writes model "<synthetic>" for locally generated notices
    // (API errors, limits). The first real model labels the session.
    let session = parse_fixture("synthetic_first.jsonl");
    assert_eq!(session.model.as_deref(), Some("claude-sonnet-4"));
    assert_eq!(session.tokens.output, 900);

    // A session that only ever saw notices has no model at all.
    let session = parse_fixture("synthetic_only.jsonl");
    assert_eq!(session.model, None);
    assert_eq!(session.tokens.output, 0);
}

#[test]
fn a_window_counts_only_the_activity_inside_it() {
    use conscience::interval::Interval;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic_session.jsonl");
    let parser = ClaudeCodeParser::new();
    let full = parser.parse_session_file("s", &path).unwrap();
    assert_eq!(full.turns.human, 2);

    // Only the second prompt (u5 at 16:41:05) and what followed it.
    let late = Interval::between(
        "2026-09-01T16:41:00Z".parse().unwrap(),
        "2026-09-01T16:42:00Z".parse().unwrap(),
    );
    let clipped = parser
        .parse_session_file_within("s", &path, Some(&late))
        .unwrap()
        .expect("has activity in window");
    assert_eq!(clipped.turns.human, 1);
    assert!(clipped.turns.assistant < full.turns.assistant);
    assert!(clipped.tokens.output < full.tokens.output);
    assert!(clipped.tokens.output > 0);
    assert_eq!(clipped.interactions.len(), 1);
    assert!(clipped.started_at.unwrap() >= late.start, "span is the in-window span");
    // Metadata is read from the whole file even when the record is outside.
    assert_eq!(clipped.project_path.as_deref(), Some("/home/dev/projA"));
    assert!(clipped.model.is_some());

    // A window the session does not touch: it did not happen then.
    let before = Interval::between(
        "2026-09-01T10:00:00Z".parse().unwrap(),
        "2026-09-01T11:00:00Z".parse().unwrap(),
    );
    assert!(parser.parse_session_file_within("s", &path, Some(&before)).unwrap().is_none());
}
