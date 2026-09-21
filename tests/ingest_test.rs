//! The merged AI ingest runs every parser, keeps per-tool bookkeeping for
//! coverage, and labels the summary by which tools contributed.

use conscience::ai_tools::claude_code::ClaudeCodeParser;
use conscience::ai_tools::codex::CodexParser;
use conscience::ai_tools::models::AiTool;
use conscience::ai_tools::parser::AiToolParser;
use conscience::ingest::ai::ingest_with;
use conscience::project::{ProjectScope, encode_project_dir};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn temp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("conscience-ingest-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// A fake ~/.claude with one session for `/work/app`, carrying that cwd.
fn fake_claude(root: &Path) {
    let dir = root
        .join("projects")
        .join(encode_project_dir(Path::new("/work/app")));
    fs::create_dir_all(&dir).unwrap();
    let body = fixture("basic_session.jsonl").replace("/home/dev/projA", "/work/app");
    fs::write(dir.join("s1.jsonl"), body).unwrap();
}

/// A fake ~/.codex with one session whose cwd is `/work/app`.
fn fake_codex(root: &Path) {
    let dir = root.join("sessions").join("2026");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("rollout.jsonl"), fixture("codex_session.jsonl")).unwrap();
}

#[test]
fn merges_both_tools_and_reports_each() {
    let claude_root = temp("claude");
    let codex_root = temp("codex");
    fake_claude(&claude_root);
    fake_codex(&codex_root);

    let claude = ClaudeCodeParser::with_dir(claude_root.clone());
    let codex = CodexParser::with_dir(codex_root.clone());
    let parsers: [(AiTool, &'static str, &dyn AiToolParser); 2] = [
        (AiTool::ClaudeCode, "claude_code", &claude),
        (AiTool::Codex, "codex", &codex),
    ];
    let scope = ProjectScope::from_root(PathBuf::from("/work/app"));

    let ingested = ingest_with(&parsers, Some(&scope), None).unwrap();

    assert_eq!(ingested.summary.tool, AiTool::Mixed);
    assert_eq!(ingested.summary.session_count, 2);
    assert_eq!(ingested.tools.len(), 2);
    let claude_t = &ingested.tools[0];
    let codex_t = &ingested.tools[1];
    assert!(claude_t.detected && codex_t.detected);
    assert_eq!(claude_t.sessions_in_range, 1);
    assert_eq!(codex_t.sessions_in_range, 1);
    assert!(
        ingested
            .summary
            .sessions
            .iter()
            .any(|s| s.tool == AiTool::Codex)
    );
    // Codex's model came through, so energy can tier it.
    assert!(
        ingested
            .summary
            .sessions
            .iter()
            .any(|s| s.model.as_deref() == Some("gpt-5.5"))
    );

    fs::remove_dir_all(&claude_root).ok();
    fs::remove_dir_all(&codex_root).ok();
}

#[test]
fn a_tool_with_no_data_is_reported_not_failed() {
    let claude_root = temp("claude-only");
    let codex_root = temp("codex-missing");
    fake_claude(&claude_root);
    // codex_root exists but has no sessions dir: not detected.

    let claude = ClaudeCodeParser::with_dir(claude_root.clone());
    let codex = CodexParser::with_dir(codex_root.clone());
    let parsers: [(AiTool, &'static str, &dyn AiToolParser); 2] = [
        (AiTool::ClaudeCode, "claude_code", &claude),
        (AiTool::Codex, "codex", &codex),
    ];
    let scope = ProjectScope::from_root(PathBuf::from("/work/app"));

    let ingested = ingest_with(&parsers, Some(&scope), None).unwrap();
    assert_eq!(
        ingested.summary.tool,
        AiTool::ClaudeCode,
        "one contributor keeps its own label"
    );
    assert_eq!(ingested.summary.session_count, 1);
    let codex_t = &ingested.tools[1];
    assert!(!codex_t.detected);
    assert!(codex_t.error.is_none());
    assert_eq!(codex_t.sessions_in_range, 0);

    fs::remove_dir_all(&claude_root).ok();
    fs::remove_dir_all(&codex_root).ok();
}

#[test]
fn scope_filters_codex_by_cwd_too() {
    let claude_root = temp("claude-scope");
    let codex_root = temp("codex-scope");
    fake_claude(&claude_root);
    fake_codex(&codex_root);

    let claude = ClaudeCodeParser::with_dir(claude_root.clone());
    let codex = CodexParser::with_dir(codex_root.clone());
    let parsers: [(AiTool, &'static str, &dyn AiToolParser); 2] = [
        (AiTool::ClaudeCode, "claude_code", &claude),
        (AiTool::Codex, "codex", &codex),
    ];
    let other = ProjectScope::from_root(PathBuf::from("/work/other"));

    let ingested = ingest_with(&parsers, Some(&other), None).unwrap();
    assert_eq!(ingested.summary.session_count, 0);
    assert!(ingested.tools.iter().all(|t| t.sessions_in_range == 0));

    fs::remove_dir_all(&claude_root).ok();
    fs::remove_dir_all(&codex_root).ok();
}

#[test]
fn codex_clips_cumulative_usage_to_the_window() {
    use conscience::interval::Interval;
    let codex_root = temp("codex-clip");
    fake_codex(&codex_root);
    let codex = CodexParser::with_dir(codex_root.clone());

    // Everything in the fixture happens 10:00:00 to 10:00:11 on 2026-09-02.
    let covering = Interval::between(
        "2026-09-02T09:00:00Z".parse().unwrap(),
        "2026-09-02T11:00:00Z".parse().unwrap(),
    );
    let s = codex.parse_within(None, Some(&covering)).unwrap();
    assert_eq!(s.session_count, 1);
    assert_eq!(s.total_tokens.output, 800, "output + reasoning output");
    assert_eq!(s.total_turns.human, 1);

    // A window ending before the usage event: turns may be in, tokens are not.
    let early = Interval::between(
        "2026-09-02T09:59:00Z".parse().unwrap(),
        "2026-09-02T10:00:05Z".parse().unwrap(),
    );
    let s = codex.parse_within(None, Some(&early)).unwrap();
    assert_eq!(s.session_count, 1);
    assert_eq!(s.total_turns.human, 1);
    assert_eq!(s.total_tokens.output, 0, "usage was recorded after the window");

    // A window before the session: it did not happen then.
    let none = Interval::between(
        "2026-09-01T00:00:00Z".parse().unwrap(),
        "2026-09-01T01:00:00Z".parse().unwrap(),
    );
    assert_eq!(codex.parse_within(None, Some(&none)).unwrap().session_count, 0);

    fs::remove_dir_all(&codex_root).ok();
}
