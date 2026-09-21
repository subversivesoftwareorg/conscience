use crate::ai_tools::models::*;
use crate::ai_tools::parser::AiToolParser;
use crate::error::{ConscienceError, Result};
use crate::interval::Interval;
use crate::project::ProjectScope;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub struct CodexParser {
    codex_dir: PathBuf,
}

impl Default for CodexParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexParser {
    pub fn new() -> Self {
        let codex_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".codex");
        Self { codex_dir }
    }

    /// Point the parser at a specific `.codex`-style directory instead of
    /// the user's home. Used by tests with fixture trees.
    pub fn with_dir(codex_dir: PathBuf) -> Self {
        Self { codex_dir }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.codex_dir.join("sessions")
    }

    fn find_session_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        Self::walk_jsonl(&self.sessions_dir(), &mut files);
        files
    }

    fn walk_jsonl(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::walk_jsonl(&path, files);
                } else if path.extension().is_some_and(|e| e == "jsonl") {
                    files.push(path);
                }
            }
        }
    }

    /// Parse one session, counting only events inside `window` when given.
    /// Codex records cumulative token usage, so the in-window figure is the
    /// last usage inside the window minus the last usage before it.
    fn parse_session_within(
        &self,
        path: &Path,
        window: Option<&Interval>,
    ) -> Result<Option<AiSession>> {
        let file = std::fs::File::open(path)
            .map_err(|e| ConscienceError::Other(anyhow::anyhow!("Failed to open {:?}: {}", path, e)))?;
        let reader = std::io::BufReader::new(file);

        let mut session_id = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mut project_path: Option<String> = None;
        let mut model: Option<String> = None;
        let mut started_at: Option<DateTime<Utc>> = None;
        let mut ended_at: Option<DateTime<Utc>> = None;
        let mut tools_used: HashMap<String, u64> = HashMap::new();
        let mut bash_commands: Vec<String> = Vec::new();
        let mut human_turns = 0u64;
        let mut assistant_turns = 0u64;
        // Cumulative usage: (input, output, cached) at the last event seen
        // before the window, and at the last event inside it.
        let mut usage_before = (0u64, 0u64, 0u64);
        let mut usage_in = (0u64, 0u64, 0u64);
        let mut dated_records = 0u64;
        let mut in_window_records = 0u64;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.is_empty() {
                continue;
            }

            let value: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let payload = value.get("payload").unwrap_or(&Value::Null);

            let dt_opt = value
                .get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|ts| ts.parse::<DateTime<Utc>>().ok());
            let in_window = match (window, dt_opt) {
                (None, _) => true,
                (Some(w), Some(dt)) => w.contains(dt),
                (Some(_), None) => false,
            };
            let before_window = match (window, dt_opt) {
                (Some(w), Some(dt)) => dt < w.start,
                _ => false,
            };
            if dt_opt.is_some() {
                dated_records += 1;
                if in_window {
                    in_window_records += 1;
                }
            }
            if let (Some(dt), true) = (dt_opt, in_window) {
                if started_at.is_none() || dt < started_at.unwrap() {
                    started_at = Some(dt);
                }
                if ended_at.is_none() || dt > ended_at.unwrap() {
                    ended_at = Some(dt);
                }
            }

            match event_type {
                "session_meta" => {
                    if let Some(id) = payload.get("id").and_then(|v| v.as_str()) {
                        session_id = id.to_string();
                    }
                    if let Some(cwd) = payload.get("cwd").and_then(|v| v.as_str()) {
                        project_path = Some(cwd.to_string());
                    }
                }
                "turn_context" => {
                    if model.is_none() {
                        model = payload
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                    }
                }
                "response_item" => {
                    if !in_window {
                        continue;
                    }
                    let item = payload.get("item").unwrap_or(payload);

                    match item.get("role").and_then(|v| v.as_str()) {
                        Some("user") => human_turns += 1,
                        Some("assistant") => assistant_turns += 1,
                        _ => {}
                    }

                    let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if item_type == "function_call" {
                        let tool_name = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        *tools_used.entry(tool_name.clone()).or_insert(0) += 1;

                        if tool_name == "exec_command" {
                            if let Some(args) = item.get("arguments").and_then(|v| v.as_str()) {
                                if let Ok(args_obj) = serde_json::from_str::<Value>(args) {
                                    if let Some(cmd) = args_obj.get("cmd").and_then(|v| v.as_str()) {
                                        bash_commands.push(cmd.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                "event_msg" => {
                    if let Some(usage) = payload.get("info").and_then(|i| i.get("total_token_usage")) {
                        let get = |k: &str| usage.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
                        let snapshot = (
                            get("input_tokens"),
                            get("output_tokens") + get("reasoning_output_tokens"),
                            get("cached_input_tokens"),
                        );
                        if before_window {
                            usage_before = snapshot;
                        } else if in_window {
                            usage_in = snapshot;
                        }
                    }
                }
                _ => {}
            }
        }

        if window.is_some() && dated_records > 0 && in_window_records == 0 {
            return Ok(None);
        }

        // In-window deltas of the cumulative counters.
        let last_input_tokens = usage_in.0.saturating_sub(usage_before.0);
        let last_output_tokens = usage_in.1.saturating_sub(usage_before.1);
        let last_cached_tokens = usage_in.2.saturating_sub(usage_before.2);
        let net_input = last_input_tokens.saturating_sub(last_cached_tokens);

        Ok(Some(AiSession {
            tool: AiTool::Codex,
            session_id,
            project_path,
            started_at,
            ended_at,
            model,
            work_categories: vec![WorkCategory::Code],
            turns: TurnCounts {
                human: human_turns,
                assistant: assistant_turns,
                machine: 0,
                total: human_turns + assistant_turns,
            },
            tokens: TokenUsage {
                input: net_input,
                output: last_output_tokens,
                cache_creation: last_input_tokens.saturating_sub(last_cached_tokens),
                cache_read: last_cached_tokens,
            },
            tools_used,
            files_touched: Vec::new(),
            bash_commands,
            agent_actions: Vec::new(),
            git_branch: None,
            interactions: Vec::new(),
            agent_dispatches: Vec::new(),
            skill_invocations: Vec::new(),
            launch: Default::default(),
        }))
    }
}

impl AiToolParser for CodexParser {
    fn tool_name(&self) -> &str {
        "OpenAI Codex"
    }

    fn detect(&self) -> bool {
        self.sessions_dir().exists()
    }

    fn data_path(&self) -> String {
        self.sessions_dir().to_string_lossy().to_string()
    }

    fn parse(&self, scope: Option<&ProjectScope>) -> Result<AiUsageSummary> {
        self.parse_within(scope, None)
    }

    fn parse_within(
        &self,
        scope: Option<&ProjectScope>,
        window: Option<&Interval>,
    ) -> Result<AiUsageSummary> {
        let session_files = self.find_session_files();
        if session_files.is_empty() {
            return Ok(AiUsageSummary::empty(AiTool::Codex));
        }

        let mut sessions = Vec::new();

        for path in &session_files {
            match self.parse_session_within(path, window) {
                Ok(None) => {}
                Ok(Some(session)) => {
                    // Codex records the working directory per session; that is
                    // the only project identity available, so scope on it.
                    if let Some(s) = scope {
                        let in_scope = session
                            .project_path
                            .as_deref()
                            .is_some_and(|cwd| s.matches_cwd(cwd));
                        if !in_scope {
                            continue;
                        }
                    }
                    sessions.push(session);
                }
                Err(e) => {
                    eprintln!("  Warning: failed to parse {:?}: {}", path, e);
                }
            }
        }

        let summary = AiUsageSummary::from_sessions(AiTool::Codex, sessions);

        Ok(summary)
    }
}
