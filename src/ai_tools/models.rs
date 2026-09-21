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
    /// A summary that merges sessions from more than one tool.
    Mixed,
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
            AiTool::Mixed => write!(f, "AI tools (mixed)"),
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

/// One human prompt and the approximate span the AI worked on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub human_at: DateTime<Utc>,
    pub ai_until: Option<DateTime<Utc>>,
    pub uuid: Option<String>,
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
    #[serde(default)]
    pub interactions: Vec<Interaction>,
    #[serde(default)]
    pub agent_dispatches: Vec<AgentDispatch>,
    #[serde(default)]
    pub skill_invocations: Vec<SkillInvocation>,
    /// How the session was started and whether it got anywhere. Lets
    /// automation be told apart from a person at a terminal.
    #[serde(default)]
    pub launch: Launch,
}

/// Launch metadata Claude Code records on each session.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Launch {
    /// `cli` for a person at a terminal, `sdk-cli` for `claude -p` and the
    /// Agent SDK, which is how cron jobs and scripts run it.
    pub entrypoint: Option<String>,
    /// `sdk` when the prompt came from a program rather than a person.
    pub prompt_source: Option<String>,
    /// The first human prompt, trimmed; the signature of a recurring job.
    pub first_prompt: Option<String>,
    /// The API error the session ended on, e.g. `authentication_failed`.
    pub api_error: Option<String>,
}

impl Launch {
    /// Started by a program rather than a person.
    pub fn is_automated(&self) -> bool {
        self.entrypoint.as_deref() == Some("sdk-cli") || self.prompt_source.as_deref() == Some("sdk")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDispatch {
    pub description: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvocation {
    pub skill: String,
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
    /// Start of the interval these totals cover; `None` means all time.
    #[serde(default)]
    pub period_start: Option<DateTime<Utc>>,
    /// End of the interval these totals cover; `None` means all time.
    #[serde(default)]
    pub period_end: Option<DateTime<Utc>>,
    /// Sessions that carried no timestamp and so could not be placed in or
    /// out of the interval. Excluded from every total above.
    #[serde(default)]
    pub undated_sessions: u64,
}

impl AiUsageSummary {
    /// An empty summary for a tool with no data.
    pub fn empty(tool: AiTool) -> Self {
        Self::from_sessions(tool, Vec::new())
    }

    /// Build every total from a list of sessions. This is the single place
    /// aggregation happens, so parsers and interval filtering agree exactly.
    pub fn from_sessions(tool: AiTool, sessions: Vec<AiSession>) -> Self {
        let mut total_tokens = TokenUsage::default();
        let mut total_turns = TurnCounts::default();
        let mut models_used: HashMap<String, u64> = HashMap::new();
        let mut tools_used: HashMap<String, u64> = HashMap::new();
        let mut unique_files: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut files_touched_count = 0u64;

        for session in &sessions {
            total_tokens.input += session.tokens.input;
            total_tokens.output += session.tokens.output;
            total_tokens.cache_creation += session.tokens.cache_creation;
            total_tokens.cache_read += session.tokens.cache_read;

            total_turns.human += session.turns.human;
            total_turns.assistant += session.turns.assistant;
            total_turns.machine += session.turns.machine;
            total_turns.total += session.turns.total;

            if let Some(m) = &session.model {
                *models_used.entry(m.clone()).or_insert(0) += 1;
            }
            for (tool_name, count) in &session.tools_used {
                *tools_used.entry(tool_name.clone()).or_insert(0) += count;
            }
            for file in &session.files_touched {
                unique_files.insert(file.path.as_str());
                files_touched_count += 1;
            }
        }

        let unique_files_touched = unique_files.len() as u64;

        let all_bash_commands: Vec<BashCommand> = sessions
            .iter()
            .flat_map(|s| {
                s.bash_commands.iter().map(|cmd| BashCommand {
                    command: cmd.clone(),
                    session_id: s.session_id.clone(),
                })
            })
            .collect();

        Self {
            tool,
            session_count: sessions.len() as u64,
            total_tokens,
            total_turns,
            models_used,
            tools_used,
            files_touched_count,
            unique_files_touched,
            all_bash_commands,
            agent_actions_summary: AgentActionsSummary::default(),
            sessions,
            period_start: None,
            period_end: None,
            undated_sessions: 0,
        }
    }

    /// A new summary containing only sessions that overlap `interval`,
    /// with every total rebuilt. Undated sessions are dropped and counted.
    pub fn restrict(&self, interval: &crate::interval::Interval) -> Self {
        let mut kept = Vec::new();
        let mut undated = 0u64;
        for s in &self.sessions {
            match interval.covers_session(s) {
                Some(true) => kept.push(s.clone()),
                Some(false) => {}
                None => undated += 1,
            }
        }
        let mut out = Self::from_sessions(self.tool.clone(), kept);
        out.period_start = Some(interval.start);
        out.period_end = Some(interval.end);
        out.undated_sessions = undated;
        out
    }
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
