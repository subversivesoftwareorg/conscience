use crate::ai_tools::models::*;
#[allow(unused_imports)]
use crate::ai_tools::models::{AgentActionsSummary, WorkCategory};
use crate::ai_tools::parser::AiToolParser;
use crate::error::{ConscienceError, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub struct ClaudeCodeParser {
    claude_dir: PathBuf,
}

impl Default for ClaudeCodeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCodeParser {
    pub fn new() -> Self {
        let claude_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude");
        Self { claude_dir }
    }

    fn projects_dir(&self) -> PathBuf {
        self.claude_dir.join("projects")
    }

    fn find_session_files(&self, project_filter: Option<&Path>) -> Vec<(String, PathBuf)> {
        let projects_dir = self.projects_dir();
        if !projects_dir.exists() {
            return Vec::new();
        }

        let mut sessions = Vec::new();

        let entries = match fs::read_dir(&projects_dir) {
            Ok(e) => e,
            Err(_) => return sessions,
        };

        for entry in entries.flatten() {
            let project_dir = entry.path();
            if !project_dir.is_dir() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();

            if let Some(filter) = project_filter {
                let decoded = decode_project_dir(&dir_name);
                if !decoded.contains(&filter.to_string_lossy().to_string()) {
                    continue;
                }
            }

            for file in fs::read_dir(&project_dir).into_iter().flatten().flatten() {
                let path = file.path();
                if path.extension().is_some_and(|e| e == "jsonl") {
                    let stem = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    sessions.push((stem, path));
                }
            }
        }

        sessions
    }

    pub fn parse_session_file(&self, session_id: &str, path: &Path) -> Result<AiSession> {
        self.parse_session(session_id, path)
    }

    fn is_human_prompt(value: &Value) -> bool {
        if value.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false) {
            return false;
        }
        if value.get("isMeta").and_then(|v| v.as_bool()).unwrap_or(false) {
            return false;
        }
        match value.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
            Some(text) => {
                !text.starts_with("Caveat:") && !text.starts_with("<local-command-stdout>")
            }
            None => false,
        }
    }

    fn parse_session(&self, session_id: &str, path: &Path) -> Result<AiSession> {
        let file = fs::File::open(path)
            .map_err(|e| ConscienceError::Other(anyhow::anyhow!("Failed to open {:?}: {}", path, e)))?;
        let reader = BufReader::new(file);

        let mut human_turns = 0u64;
        let mut assistant_turns = 0u64;
        let mut machine_turns = 0u64;
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut cache_creation = 0u64;
        let mut cache_read = 0u64;
        let mut tools_used: HashMap<String, u64> = HashMap::new();
        let mut files_touched: Vec<FileTouched> = Vec::new();
        let mut files_seen: HashSet<String> = HashSet::new();
        let mut bash_commands: Vec<String> = Vec::new();
        let mut model: Option<String> = None;
        let mut project_path: Option<String> = None;
        let mut git_branch: Option<String> = None;
        let mut started_at: Option<DateTime<Utc>> = None;
        let mut ended_at: Option<DateTime<Utc>> = None;

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

            let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

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

            match msg_type {
                "user" => {
                    if value.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false) {
                        // Sidechain user events are neither human nor machine
                    } else if Self::is_human_prompt(&value) {
                        human_turns += 1;
                    } else {
                        machine_turns += 1;
                    }
                    if project_path.is_none() {
                        project_path = value
                            .get("cwd")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                    if git_branch.is_none() {
                        git_branch = value
                            .get("gitBranch")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }
                "assistant" => {
                    if value.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false) {
                        continue;
                    }
                    assistant_turns += 1;

                    let msg = value.get("message").unwrap_or(&Value::Null);

                    if model.is_none() {
                        model = msg
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }

                    if let Some(usage) = msg.get("usage") {
                        input_tokens += usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        output_tokens += usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        cache_creation += usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        cache_read += usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }

                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                let tool_name = block
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                *tools_used.entry(tool_name.clone()).or_insert(0) += 1;

                                if tool_name == "Bash" {
                                    if let Some(cmd) = block
                                        .get("input")
                                        .and_then(|i| i.get("command"))
                                        .and_then(|c| c.as_str())
                                    {
                                        bash_commands.push(cmd.to_string());
                                    }
                                }

                                extract_file_touch(&tool_name, block, &mut files_touched, &mut files_seen);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(AiSession {
            tool: AiTool::ClaudeCode,
            session_id: session_id.to_string(),
            project_path,
            started_at,
            ended_at,
            model,
            work_categories: vec![WorkCategory::Code],
            turns: TurnCounts {
                human: human_turns,
                assistant: assistant_turns,
                machine: machine_turns,
                total: human_turns + assistant_turns,
            },
            tokens: TokenUsage {
                input: input_tokens,
                output: output_tokens,
                cache_creation,
                cache_read,
            },
            tools_used,
            files_touched,
            bash_commands,
            agent_actions: Vec::new(),
            git_branch,
        })
    }
}

impl AiToolParser for ClaudeCodeParser {
    fn tool_name(&self) -> &str {
        "Claude Code"
    }

    fn detect(&self) -> bool {
        self.projects_dir().exists()
    }

    fn data_path(&self) -> String {
        self.projects_dir().to_string_lossy().to_string()
    }

    fn parse(&self, project_filter: Option<&Path>) -> Result<AiUsageSummary> {
        let session_files = self.find_session_files(project_filter);
        let mut sessions = Vec::new();
        let mut total_tokens = TokenUsage::default();
        let mut total_turns = TurnCounts::default();
        let mut models_used: HashMap<String, u64> = HashMap::new();
        let mut all_tools_used: HashMap<String, u64> = HashMap::new();
        let mut all_files: HashSet<String> = HashSet::new();
        let mut total_files_touched = 0u64;

        for (session_id, path) in &session_files {
            match self.parse_session(session_id, path) {
                Ok(session) => {
                    total_tokens.input += session.tokens.input;
                    total_tokens.output += session.tokens.output;
                    total_tokens.cache_creation += session.tokens.cache_creation;
                    total_tokens.cache_read += session.tokens.cache_read;

                    total_turns.human += session.turns.human;
                    total_turns.assistant += session.turns.assistant;
                    total_turns.machine += session.turns.machine;
                    total_turns.total += session.turns.total;

                    if let Some(ref m) = session.model {
                        *models_used.entry(m.clone()).or_insert(0) += 1;
                    }

                    for (tool, count) in &session.tools_used {
                        *all_tools_used.entry(tool.clone()).or_insert(0) += count;
                    }

                    for file in &session.files_touched {
                        all_files.insert(file.path.clone());
                        total_files_touched += 1;
                    }

                    sessions.push(session);
                }
                Err(e) => {
                    eprintln!("Warning: failed to parse session {}: {}", session_id, e);
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

        Ok(AiUsageSummary {
            tool: AiTool::ClaudeCode,
            session_count: sessions.len() as u64,
            total_tokens,
            total_turns,
            models_used,
            tools_used: all_tools_used,
            files_touched_count: total_files_touched,
            unique_files_touched: all_files.len() as u64,
            all_bash_commands,
            agent_actions_summary: AgentActionsSummary::default(),
            sessions,
        })
    }
}

fn extract_file_touch(
    tool_name: &str,
    block: &Value,
    files: &mut Vec<FileTouched>,
    seen: &mut HashSet<String>,
) {
    let input = block.get("input").unwrap_or(&Value::Null);

    let (path_key, action) = match tool_name {
        "Read" => ("file_path", FileAction::Read),
        "Write" => ("file_path", FileAction::Write),
        "Edit" => ("file_path", FileAction::Edit),
        _ => return,
    };

    if let Some(path) = input.get(path_key).and_then(|v| v.as_str()) {
        if seen.insert(path.to_string()) {
            files.push(FileTouched {
                path: path.to_string(),
                action,
            });
        }
    }
}

pub fn decode_project_dir(dir_name: &str) -> String {
    dir_name.replace('-', "/")
}
