//! Project scoping against a fixture `.claude/projects` tree.
//!
//! Builds two project directories whose encoded names share a prefix
//! (the case that broke containment matching) plus a dotted path whose
//! encoded name collides with a dashed one, then checks that scoping
//! picks exactly the right sessions.

use conscience::ai_tools::claude_code::ClaudeCodeParser;
use conscience::ai_tools::parser::AiToolParser;
use conscience::project::{ProjectScope, encode_project_dir};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_session() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic_session.jsonl"),
    )
    .expect("fixture exists")
}

/// Create a fake `.claude` dir with one session per given project path.
fn fake_claude_dir(tag: &str, projects: &[&str]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("conscience-scope-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let body = fixture_session();
    for p in projects {
        let dir = root.join("projects").join(encode_project_dir(Path::new(p)));
        fs::create_dir_all(&dir).unwrap();
        // Prepend a record carrying the real cwd, as Claude Code does.
        let with_cwd = format!(
            "{{\"type\":\"user\",\"cwd\":\"{}\",\"isMeta\":true,\"message\":{{\"role\":\"user\",\"content\":\"\"}}}}\n{}",
            p, body
        );
        fs::write(dir.join("session.jsonl"), with_cwd).unwrap();
    }
    root
}

#[test]
fn scope_matches_only_the_exact_project_not_prefix_siblings() {
    let root = fake_claude_dir(
        "prefix",
        &["/work/conscience", "/work/conscience-dashboard", "/work"],
    );
    let parser = ClaudeCodeParser::with_dir(root.clone());

    let scope = ProjectScope::from_root(PathBuf::from("/work/conscience"));
    let dirs = parser.matching_project_dirs(&scope);
    assert_eq!(dirs, vec!["-work-conscience".to_string()]);

    let summary = parser.parse(Some(&scope)).unwrap();
    assert_eq!(summary.session_count, 1, "one session, not three");

    let all = parser.parse(None).unwrap();
    assert_eq!(all.session_count, 3, "no scope means every project");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn worktrees_declared_in_scope_are_included() {
    let root = fake_claude_dir("wt", &["/work/app", "/work/app-wt-feature", "/work/other"]);
    let parser = ClaudeCodeParser::with_dir(root.clone());

    let mut scope = ProjectScope::from_root(PathBuf::from("/work/app"));
    scope.worktrees.push(PathBuf::from("/work/app-wt-feature"));

    let mut dirs = parser.matching_project_dirs(&scope);
    dirs.sort();
    assert_eq!(
        dirs,
        vec!["-work-app".to_string(), "-work-app-wt-feature".to_string()]
    );
    assert_eq!(parser.parse(Some(&scope)).unwrap().session_count, 2);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn project_root_comes_from_session_cwd_not_decoded_name() {
    // "general.legal" and "general-legal" encode identically; only the
    // cwd recorded inside the session can tell us which one it really is.
    let root = fake_claude_dir("dotted", &["/work/kondasecurity/general.legal"]);
    let parser = ClaudeCodeParser::with_dir(root.clone());

    let real = parser.project_root_for_dir("-work-kondasecurity-general-legal");
    assert_eq!(real, "/work/kondasecurity/general.legal");

    // The lossy decode would have said "/work/kondasecurity/general/legal".
    assert_ne!(
        real,
        conscience::ai_tools::claude_code::decode_project_dir("-work-kondasecurity-general-legal")
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn empty_project_dir_falls_back_to_decoded_name() {
    let root = std::env::temp_dir().join(format!("conscience-scope-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("projects").join("-work-plain")).unwrap();
    let parser = ClaudeCodeParser::with_dir(root.clone());

    assert_eq!(parser.project_root_for_dir("-work-plain"), "/work/plain");

    fs::remove_dir_all(&root).ok();
}
