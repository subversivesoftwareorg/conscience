use crate::ai_tools::claude_code::ClaudeCodeParser;
use crate::ai_tools::models::AiUsageSummary;
use crate::ai_tools::parser::AiToolParser;
use crate::error::Result;
use crate::interval::Interval;
use crate::project::ProjectScope;

/// Ingest Claude Code sessions.
///
/// `scope`: `None` scans every project; `Some` restricts to the resolved
/// project and its worktrees.
/// `interval`: `None` means all time; `Some` keeps only sessions that
/// overlap it and reports undated sessions separately.
pub fn ingest_claude_code(
    scope: Option<&ProjectScope>,
    interval: Option<&Interval>,
) -> Result<AiUsageSummary> {
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
        for wt in &s.worktrees {
            eprintln!("  + worktree {}", wt.display());
        }
    } else {
        eprintln!("Project: all Claude Code projects");
    }

    let all = parser.parse(scope)?;

    let summary = match interval {
        Some(iv) => {
            let restricted = all.restrict(iv);
            eprintln!(
                "Interval: {}; {} of {} session(s) in range{}",
                iv.label(),
                restricted.session_count,
                all.session_count,
                if restricted.undated_sessions > 0 {
                    format!(", {} undated excluded", restricted.undated_sessions)
                } else {
                    String::new()
                }
            );
            restricted
        }
        None => {
            eprintln!("Interval: all time ({} session(s))", all.session_count);
            all
        }
    };

    eprintln!(
        "Found {} session(s), {} total turns, {} output tokens",
        summary.session_count, summary.total_turns.total, summary.total_tokens.output,
    );

    Ok(summary)
}
