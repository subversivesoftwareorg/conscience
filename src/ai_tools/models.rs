use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Which AI tool produced this session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AiTool {
    ClaudeCode,
    Copilot,
    Cursor,
    Codex,
    Windsurf,
    OpenClaw,
    NanoClaw,
    Other(String),
}

impl std::fmt::Display for AiTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiTool::ClaudeCode => write!(f, "Claude Code"),
            AiTool::Copilot => write!(f, "GitHub Copilot"),
            AiTool::Cursor => write!(f, "Cursor"),
            AiTool::Codex => write!(f, "Codex"),
            AiTool::Windsurf => write!(f, "Windsurf"),
            AiTool::OpenClaw => write!(f, "OpenClaw"),
            AiTool::NanoClaw => write!(f, "NanoClaw"),
            AiTool::Other(name) => write!(f, "{}", name),
        }
    }
}

/// What kind of work this AI session involved
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkCategory {
    Code,
    Communication,
    Scheduling,
    Research,
    Writing,
    DataProcessing,
    Automation,
    Other(String),
}

impl std::fmt::Display for WorkCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkCategory::Code => write!(f, "Code"),
            WorkCategory::Communication => write!(f, "Communication"),
            WorkCategory::Scheduling => write!(f, "Scheduling"),
            WorkCategory::Research => write!(f, "Research"),
            WorkCategory::Writing => write!(f, "Writing"),
            WorkCategory::DataProcessing => write!(f, "Data Processing"),
            WorkCategory::Automation => write!(f, "Automation"),
            WorkCategory::Other(name) => write!(f, "{}", name),
        }
    }
}

/// A single session of AI-assisted work, normalized across all tools.
///
/// This is the common currency of Conscience — every tool-specific parser
/// produces these. The model captures what we can measure without storing
/// the actual conversation content (privacy by design).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSession {
    pub tool: AiTool,
    pub session_id: String,
    pub project_path: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub model: Option<String>,
    pub work_categories: Vec<WorkCategory>,
    pub turns: TurnCounts,
    pub tokens: TokenUsage,
    pub tools_used: HashMap<String, u64>,
    pub files_touched: Vec<FileTouched>,
    pub bash_commands: Vec<String>,
    pub agent_actions: Vec<AgentAction>,
    pub git_branch: Option<String>,
}

/// An action taken by an autonomous AI agent that affects the outside world.
/// These are distinct from tool invocations (which affect the local machine)
/// because they have a *recipient* — another human who is affected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub action_type: AgentActionType,
    pub channel: String,
    pub required_approval: bool,
    pub was_approved: Option<bool>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionType {
    MessageSent,
    EmailSent,
    MeetingScheduled,
    MeetingCancelled,
    CalendarModified,
    DocumentCreated,
    DocumentEdited,
    TaskCreated,
    ApprovalRequested,
    WebBrowsed,
    ApiCalled,
    FileUploaded,
    Other(String),
}

impl std::fmt::Display for AgentActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentActionType::MessageSent => write!(f, "Message Sent"),
            AgentActionType::EmailSent => write!(f, "Email Sent"),
            AgentActionType::MeetingScheduled => write!(f, "Meeting Scheduled"),
            AgentActionType::MeetingCancelled => write!(f, "Meeting Cancelled"),
            AgentActionType::CalendarModified => write!(f, "Calendar Modified"),
            AgentActionType::DocumentCreated => write!(f, "Document Created"),
            AgentActionType::DocumentEdited => write!(f, "Document Edited"),
            AgentActionType::TaskCreated => write!(f, "Task Created"),
            AgentActionType::ApprovalRequested => write!(f, "Approval Requested"),
            AgentActionType::WebBrowsed => write!(f, "Web Browsed"),
            AgentActionType::ApiCalled => write!(f, "API Called"),
            AgentActionType::FileUploaded => write!(f, "File Uploaded"),
            AgentActionType::Other(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnCounts {
    pub human: u64,
    pub assistant: u64,
    #[serde(default)]
    pub machine: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_creation + self.cache_read
    }
}

/// A file that was read, written, or edited during the session.
/// We store the path and action but not the content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTouched {
    pub path: String,
    pub action: FileAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Read,
    Write,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashCommand {
    pub command: String,
    pub session_id: String,
}

/// Aggregated summary across multiple sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiUsageSummary {
    pub tool: AiTool,
    pub session_count: u64,
    pub total_tokens: TokenUsage,
    pub total_turns: TurnCounts,
    pub models_used: HashMap<String, u64>,
    pub tools_used: HashMap<String, u64>,
    pub files_touched_count: u64,
    pub unique_files_touched: u64,
    pub all_bash_commands: Vec<BashCommand>,
    pub agent_actions_summary: AgentActionsSummary,
    pub sessions: Vec<AiSession>,
}

/// Aggregate counts of agent actions across all sessions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentActionsSummary {
    pub total_actions: u64,
    pub messages_sent: u64,
    pub emails_sent: u64,
    pub meetings_scheduled: u64,
    pub documents_created: u64,
    pub approvals_requested: u64,
    pub approvals_granted: u64,
    pub approvals_denied: u64,
    pub actions_without_approval: u64,
    pub channels_used: Vec<String>,
}
