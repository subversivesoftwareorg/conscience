use crate::ai_tools::claude_code::ClaudeCodeParser;
use crate::ai_tools::models::AiUsageSummary;
use crate::ai_tools::parser::AiToolParser;
use crate::error::Result;
use std::path::Path;

pub fn ingest_claude_code(project_filter: Option<&Path>) -> Result<AiUsageSummary> {
    let parser = ClaudeCodeParser::new();

    if !parser.detect() {
        eprintln!("No Claude Code data found at {}", parser.data_path());
        eprintln!("Claude Code stores session logs in ~/.claude/projects/");
    } else {
        eprintln!("Scanning Claude Code logs at {}", parser.data_path());
    }

    if let Some(filter) = project_filter {
        eprintln!("Filtering to project: {}", filter.display());
    }

    let summary = parser.parse(project_filter)?;

    eprintln!(
        "Found {} session(s), {} total turns, {} output tokens",
        summary.session_count,
        summary.total_turns.total,
        summary.total_tokens.output,
    );

    Ok(summary)
}
