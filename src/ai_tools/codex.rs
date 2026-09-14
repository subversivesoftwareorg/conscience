use crate::ai_tools::models::*;
use crate::ai_tools::parser::AiToolParser;
use crate::error::{ConscienceError, Result};
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

    fn parse_session(&self, path: &Path) -> Result<AiSession> {
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
        let mut last_input_tokens = 0u64;
        let mut last_output_tokens = 0u64;
        let mut last_cached_tokens = 0u64;

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

            if let Some(ts) = value.get("timestamp").and_then(|v| v.as_str()) {
                if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
                    if started_at.is_none() || dt < started_at.unwrap() {
                        started_at = Some(dt);
                    }
                    if ended_at.is_none() || dt > ended_at.unwrap() {
                        ended_at = Some(dt);
                    }
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
                    if let Some(info) = payload.get("info") {
                        if let Some(usage) = info.get("total_token_usage") {
                            last_input_tokens = usage
                                .get("input_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            last_output_tokens = usage
                                .get("output_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0)
                                + usage
                                    .get("reasoning_output_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                            last_cached_tokens = usage
                                .get("cached_input_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
        }

        let net_input = last_input_tokens.saturating_sub(last_cached_tokens);

        Ok(AiSession {
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
        })
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

    fn parse(&self, _project_filter: Option<&Path>) -> Result<AiUsageSummary> {
        let session_files = self.find_session_files();
        if session_files.is_empty() {
            return Ok(AiUsageSummary {
                tool: AiTool::Codex,
                session_count: 0,
                total_tokens: TokenUsage::default(),
                total_turns: TurnCounts::default(),
                models_used: HashMap::new(),
                tools_used: HashMap::new(),
                files_touched_count: 0,
                unique_files_touched: 0,
                all_bash_commands: Vec::new(),
                agent_actions_summary: AgentActionsSummary::default(),
                sessions: Vec::new(),
            });
        }

        eprintln!("Scanning Codex sessions at {}", self.sessions_dir().display());

        let mut sessions = Vec::new();
        let mut total_tokens = TokenUsage::default();
        let mut total_turns = TurnCounts::default();
        let mut models_used: HashMap<String, u64> = HashMap::new();
        let mut all_tools_used: HashMap<String, u64> = HashMap::new();

        for path in &session_files {
            match self.parse_session(path) {
                Ok(session) => {
                    total_tokens.input += session.tokens.input;
                    total_tokens.output += session.tokens.output;
                    total_tokens.cache_creation += session.tokens.cache_creation;
                    total_tokens.cache_read += session.tokens.cache_read;
                    total_turns.human += session.turns.human;
                    total_turns.assistant += session.turns.assistant;
                    total_turns.total += session.turns.total;

                    if let Some(ref m) = session.model {
                        *models_used.entry(m.clone()).or_insert(0) += 1;
                    }
                    for (tool, count) in &session.tools_used {
                        *all_tools_used.entry(tool.clone()).or_insert(0) += count;
                    }

                    sessions.push(session);
                }
                Err(e) => {
                    eprintln!("  Warning: failed to parse {:?}: {}", path, e);
                }
            }
        }

        let all_bash_commands: Vec<BashCommand> = sessions
            .iter()
            .flat_map(|s| {
                s.bash_commands.iter().map(|cmd| BashCommand {
                    command: cmd.clone(),
                    session_id: s.session_id.clone(),
                })
            })
            .collect();

        eprintln!(
            "Found {} Codex session(s), {} total turns, {} output tokens",
            sessions.len(),
            total_turns.total,
            total_tokens.output,
        );

        Ok(AiUsageSummary {
            tool: AiTool::Codex,
            session_count: sessions.len() as u64,
            total_tokens,
            total_turns,
            models_used,
            tools_used: all_tools_used,
            files_touched_count: 0,
            unique_files_touched: 0,
            all_bash_commands,
            agent_actions_summary: AgentActionsSummary::default(),
            sessions,
        })
    }
}
