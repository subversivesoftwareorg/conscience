use crate::ai_tools::models::*;
#[allow(unused_imports)]
use crate::ai_tools::models::{AgentActionsSummary, WorkCategory};
use crate::ai_tools::parser::AiToolParser;
use crate::error::{ConscienceError, Result};
use crate::project::ProjectScope;
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

    /// Point the parser at a specific `.claude`-style directory instead of
    /// the user's home. Used by tests with fixture trees.
    pub fn with_dir(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    fn projects_dir(&self) -> PathBuf {
        self.claude_dir.join("projects")
    }

    /// Names of the per-project directories under `~/.claude/projects` that
    /// belong to `scope`, matched exactly by Claude's own path encoding.
    pub fn matching_project_dirs(&self, scope: &ProjectScope) -> Vec<String> {
        let projects_dir = self.projects_dir();
        let entries = match fs::read_dir(&projects_dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| scope.matches_encoded_dir(name))
            .collect()
    }

    /// Recover the real filesystem path for a Claude project directory.
    ///
    /// The directory name is a lossy encoding, so the authoritative source is
    /// the `cwd` field recorded in the sessions themselves. Falls back to
    /// decoding the name when no session carries a cwd.
    pub fn project_root_for_dir(&self, dir_name: &str) -> String {
        let dir = self.projects_dir().join(dir_name);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.extension().is_some_and(|e| e == "jsonl") {
                    continue;
                }
                if let Some(cwd) = Self::first_cwd(&path) {
                    return cwd;
                }
            }
        }
        decode_project_dir(dir_name)
    }

    fn first_cwd(path: &Path) -> Option<String> {
        let file = fs::File::open(path).ok()?;
        let reader = BufReader::new(file);
        // The cwd appears in the first few records; don't read whole files.
        for line in reader.lines().take(50).flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                if let Some(cwd) = value.get("cwd").and_then(|v| v.as_str()) {
                    return Some(cwd.to_string());
                }
            }
        }
        None
    }

    fn find_session_files(&self, scope: Option<&ProjectScope>) -> Vec<(String, PathBuf)> {
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

            if let Some(s) = scope {
                if !s.matches_encoded_dir(&dir_name) {
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
        let mut interactions: Vec<Interaction> = Vec::new();
        let mut last_event_ts: Option<DateTime<Utc>> = None;
        let mut agent_dispatches: Vec<AgentDispatch> = Vec::new();
        let mut skill_invocations: Vec<SkillInvocation> = Vec::new();

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

                    if msg_type == "user" && Self::is_human_prompt(&value) {
                        if let Some(open) = interactions.last_mut() {
                            if open.ai_until.is_none() {
                                open.ai_until = last_event_ts;
                            }
                        }
                        interactions.push(Interaction {
                            human_at: dt,
                            ai_until: None,
                            uuid: value
                                .get("uuid")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                        });
                    }

                    last_event_ts = Some(dt);
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

                    // Claude Code writes `"<synthetic>"` for messages it
                    // generates locally (API errors, usage limits, refusals).
                    // No model ran, so it must not label the session.
                    if model.is_none() {
                        model = msg
                            .get("model")
                            .and_then(|v| v.as_str())
                            .filter(|m| *m != "<synthetic>")
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

                                if tool_name == "Agent" {
                                    let input = block.get("input").unwrap_or(&Value::Null);
                                    agent_dispatches.push(AgentDispatch {
                                        description: input.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        agent_type: input.get("subagent_type").and_then(|v| v.as_str()).map(String::from),
                                    });
                                }

                                if tool_name == "Skill" {
                                    let input = block.get("input").unwrap_or(&Value::Null);
                                    skill_invocations.push(SkillInvocation {
                                        skill: input.get("skill").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(open) = interactions.last_mut() {
            if open.ai_until.is_none() {
                open.ai_until = last_event_ts;
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
            interactions,
            agent_dispatches,
            skill_invocations,
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

    fn parse(&self, scope: Option<&ProjectScope>) -> Result<AiUsageSummary> {
        let session_files = self.find_session_files(scope);
        let mut sessions = Vec::new();

        for (session_id, path) in &session_files {
            match self.parse_session(session_id, path) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    eprintln!("Warning: failed to parse session {}: {}", session_id, e);
                }
            }
        }

        Ok(AiUsageSummary::from_sessions(AiTool::ClaudeCode, sessions))
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

/// Best-effort inverse of Claude's directory encoding.
///
/// Lossy: every `-` becomes `/`, so a path that contained a dash, dot, or
/// underscore decodes wrongly. Only use as a fallback when no session in the
/// directory recorded its real `cwd`; see `project_root_for_dir`.
pub fn decode_project_dir(dir_name: &str) -> String {
    dir_name.replace('-', "/")
}
