use crate::ai_tools::claude_code::ClaudeCodeParser;
use crate::ai_tools::codex::CodexParser;
use crate::ai_tools::models::{AiTool, AiUsageSummary};
use crate::ai_tools::parser::AiToolParser;
use crate::error::Result;
use crate::interval::Interval;
use crate::project::ProjectScope;

/// What one tool's parser contributed to a merged ingest.
#[derive(Debug, Clone)]
pub struct ToolIngest {
    pub tool: AiTool,
    /// Short key used in coverage, e.g. `claude_code`, `codex`.
    pub key: &'static str,
    /// Whether the tool's data directory exists at all.
    pub detected: bool,
    /// Sessions found for the scope before the interval was applied.
    pub sessions_all: u64,
    /// Sessions inside the interval (or all of them when there is none).
    pub sessions_in_range: u64,
    pub undated: u64,
    /// The parser failed; the message is for the coverage line.
    pub error: Option<String>,
}

/// A merged summary across every AI tool with data, plus per-tool detail.
#[derive(Debug, Clone)]
pub struct AiIngest {
    pub summary: AiUsageSummary,
    pub tools: Vec<ToolIngest>,
}

/// Print where the logs live once per process, not once per project when
/// `examine --all` walks all of them.
fn note_location_once(parser: &ClaudeCodeParser) {
    static NOTED: std::sync::Once = std::sync::Once::new();
    NOTED.call_once(|| {
        if !parser.detect() {
            eprintln!("No Claude Code data found at {}", parser.data_path());
            eprintln!("Claude Code stores session logs in ~/.claude/projects/");
        } else {
            eprintln!("Scanning Claude Code logs at {}", parser.data_path());
        }
        let codex = CodexParser::new();
        if codex.detect() {
            eprintln!("Scanning Codex sessions at {}", codex.data_path());
        }
    });
}

fn describe_scope(parser: &ClaudeCodeParser, scope: Option<&ProjectScope>) {
    match scope {
        Some(s) => {
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
        }
        None => eprintln!("Project: all Claude Code projects"),
    }
}

/// Run one parser and reduce its result to sessions plus bookkeeping.
fn run_parser(
    tool: AiTool,
    key: &'static str,
    parser: &dyn AiToolParser,
    scope: Option<&ProjectScope>,
    interval: Option<&Interval>,
) -> (Vec<crate::ai_tools::models::AiSession>, ToolIngest) {
    let detected = parser.detect();
    if !detected {
        return (
            Vec::new(),
            ToolIngest {
                tool,
                key,
                detected,
                sessions_all: 0,
                sessions_in_range: 0,
                undated: 0,
                error: None,
            },
        );
    }
    match parser.parse_within(scope, interval) {
        Ok(all) => {
            let sessions_all = all.session_count;
            let restricted = match interval {
                Some(iv) => all.restrict(iv),
                None => all,
            };
            let info = ToolIngest {
                tool,
                key,
                detected,
                sessions_all,
                sessions_in_range: restricted.session_count,
                undated: restricted.undated_sessions,
                error: None,
            };
            (restricted.sessions, info)
        }
        Err(e) => (
            Vec::new(),
            ToolIngest {
                tool,
                key,
                detected,
                sessions_all: 0,
                sessions_in_range: 0,
                undated: 0,
                error: Some(e.to_string()),
            },
        ),
    }
}

/// Ingest sessions from every AI tool with data on this machine: Claude
/// Code and Codex today. `scope` and `interval` apply to all of them.
pub fn ingest_ai(scope: Option<&ProjectScope>, interval: Option<&Interval>) -> Result<AiIngest> {
    let claude = ClaudeCodeParser::new();
    note_location_once(&claude);
    describe_scope(&claude, scope);
    let codex = CodexParser::new();
    ingest_with(
        &[
            (AiTool::ClaudeCode, "claude_code", &claude),
            (AiTool::Codex, "codex", &codex),
        ],
        scope,
        interval,
    )
}

/// The merge itself, over any set of parsers. Tests point parsers at
/// fixture directories; `ingest_ai` points them at the user's home.
pub fn ingest_with(
    parsers: &[(AiTool, &'static str, &dyn AiToolParser)],
    scope: Option<&ProjectScope>,
    interval: Option<&Interval>,
) -> Result<AiIngest> {
    let mut sessions = Vec::new();
    let mut tools = Vec::new();
    for (tool, key, parser) in parsers {
        let (mut s, info) = run_parser(tool.clone(), key, *parser, scope, interval);
        sessions.append(&mut s);
        tools.push(info);
    }

    let contributing: Vec<&ToolIngest> = tools.iter().filter(|t| t.sessions_in_range > 0).collect();
    let tool = match contributing.as_slice() {
        [] => AiTool::ClaudeCode,
        [one] => one.tool.clone(),
        _ => AiTool::Mixed,
    };
    let mut summary = AiUsageSummary::from_sessions(tool, sessions);
    if let Some(iv) = interval {
        summary.period_start = Some(iv.start);
        summary.period_end = Some(iv.end);
    }
    summary.undated_sessions = tools.iter().map(|t| t.undated).sum();

    match interval {
        Some(iv) => eprintln!(
            "Interval: {}; {}",
            iv.label(),
            tools
                .iter()
                .filter(|t| t.detected)
                .map(|t| {
                    let mut s = format!(
                        "{}: {} of {} session(s)",
                        t.key, t.sessions_in_range, t.sessions_all
                    );
                    if t.undated > 0 {
                        s.push_str(&format!(" ({} undated)", t.undated));
                    }
                    s
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None => eprintln!(
            "Interval: all time; {}",
            tools
                .iter()
                .filter(|t| t.detected)
                .map(|t| format!("{}: {} session(s)", t.key, t.sessions_in_range))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
    eprintln!(
        "Found {} session(s), {} total turns, {} output tokens",
        summary.session_count, summary.total_turns.total, summary.total_tokens.output,
    );

    Ok(AiIngest { summary, tools })
}

/// Claude Code only, for the per-tool `report ai` view.
pub fn ingest_claude_code(
    scope: Option<&ProjectScope>,
    interval: Option<&Interval>,
) -> Result<AiUsageSummary> {
    let parser = ClaudeCodeParser::new();
    note_location_once(&parser);
    describe_scope(&parser, scope);
    let all = parser.parse(scope)?;
    let summary = match interval {
        Some(iv) => all.restrict(iv),
        None => all,
    };
    eprintln!(
        "Found {} session(s), {} total turns, {} output tokens",
        summary.session_count, summary.total_turns.total, summary.total_tokens.output,
    );
    Ok(summary)
}
