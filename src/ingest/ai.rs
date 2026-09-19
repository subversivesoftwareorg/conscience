use crate::ai_tools::claude_code::ClaudeCodeParser;
use crate::ai_tools::models::AiUsageSummary;
use crate::ai_tools::parser::AiToolParser;
use crate::error::Result;
use crate::project::ProjectScope;

/// Ingest Claude Code sessions. `None` scans every project; `Some(scope)`
/// restricts to the resolved project and its worktrees.
pub fn ingest_claude_code(scope: Option<&ProjectScope>) -> Result<AiUsageSummary> {
    let parser = ClaudeCodeParser::new();

    if !parser.detect() {
        eprintln!("No Claude Code data found at {}", parser.data_path());
        eprintln!("Claude Code stores session logs in ~/.claude/projects/");
    } else {
        eprintln!("Scanning Claude Code logs at {}", parser.data_path());
    }

    if let Some(s) = scope {
        let matched = parser.matching_project_dirs(s);
        eprintln!(
            "Project: {} ({} session director{} matched)",
            s.root.display(),
            matched.len(),
            if matched.len() == 1 { "y" } else { "ies" }
        );
        if !s.worktrees.is_empty() {
            for wt in &s.worktrees {
                eprintln!("  + worktree {}", wt.display());
            }
        }
    } else {
        eprintln!("Project: all Claude Code projects");
    }

    let summary = parser.parse(scope)?;

    eprintln!(
        "Found {} session(s), {} total turns, {} output tokens",
        summary.session_count,
        summary.total_turns.total,
        summary.total_tokens.output,
    );

    Ok(summary)
}
