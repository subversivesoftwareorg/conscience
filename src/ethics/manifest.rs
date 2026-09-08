use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The conscience.yaml manifest — human-provided context that
/// bridges automated metrics with ethical meaning.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub project: ProjectInfo,
    #[serde(default)]
    pub github: GitHubInfo,
    #[serde(default)]
    pub value_categories: Vec<ValueCategory>,
    #[serde(default)]
    pub team: TeamInfo,
    #[serde(default)]
    pub monthly_review: MonthlyReview,
    #[serde(default)]
    pub thresholds: Thresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubInfo {
    pub repo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mission: String,
    #[serde(default)]
    pub beneficiaries: Vec<Stakeholder>,
    #[serde(default)]
    pub cost_bearers: Vec<Stakeholder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stakeholder {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueCategory {
    pub name: String,
    pub value_type: ValueType,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Revenue,
    Cost,
    Safety,
    Growth,
    HumanGood,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::Revenue => write!(f, "Revenue"),
            ValueType::Cost => write!(f, "Cost Reduction"),
            ValueType::Safety => write!(f, "Safety / Risk"),
            ValueType::Growth => write!(f, "Growth / Capability"),
            ValueType::HumanGood => write!(f, "Human Good"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamInfo {
    #[serde(default)]
    pub size: u32,
    #[serde(default)]
    pub roles: TeamRoles,
    #[serde(default)]
    pub learning_goals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamRoles {
    #[serde(default)]
    pub senior: u32,
    #[serde(default)]
    pub mid: u32,
    #[serde(default)]
    pub junior: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonthlyReview {
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub value_delivered: String,
    #[serde(default)]
    pub revenue_impact: String,
    #[serde(default)]
    pub growth_observations: String,
    #[serde(default)]
    pub ethical_notes: String,
    #[serde(default)]
    pub ai_sentiment: String,
}

impl MonthlyReview {
    pub fn is_populated(&self) -> bool {
        !self.value_delivered.is_empty() || !self.ai_sentiment.is_empty()
    }

    pub fn is_stale(&self, today: &str) -> bool {
        if self.last_updated.is_empty() {
            return true;
        }
        // Compare YYYY-MM; if different month, it's stale
        if self.last_updated.len() >= 7 && today.len() >= 7 {
            self.last_updated[..7] != today[..7]
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionThresholds {
    #[serde(default = "default_idle_minutes")]
    pub idle_minutes: f64,
    #[serde(default = "default_engagement_floor_minutes")]
    pub engagement_floor_minutes: f64,
    #[serde(default = "default_flow_gap_minutes")]
    pub flow_gap_minutes: f64,
    #[serde(default = "default_flow_min_minutes")]
    pub flow_min_minutes: f64,
    #[serde(default)]
    pub project_aliases: BTreeMap<String, String>,
}

impl Default for AttentionThresholds {
    fn default() -> Self {
        Self {
            idle_minutes: default_idle_minutes(),
            engagement_floor_minutes: default_engagement_floor_minutes(),
            flow_gap_minutes: default_flow_gap_minutes(),
            flow_min_minutes: default_flow_min_minutes(),
            project_aliases: BTreeMap::new(),
        }
    }
}

fn default_idle_minutes() -> f64 { 15.0 }
fn default_engagement_floor_minutes() -> f64 { 2.0 }
fn default_flow_gap_minutes() -> f64 { 10.0 }
fn default_flow_min_minutes() -> f64 { 20.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    #[serde(default = "default_concentration_warn")]
    pub contribution_concentration_warn: f64,
    #[serde(default = "default_concentration_concern")]
    pub contribution_concentration_concern: f64,
    #[serde(default = "default_ai_dependency_concern")]
    pub ai_dependency_concern: f64,
    #[serde(default = "default_ai_dependency_info")]
    pub ai_dependency_info: f64,
    #[serde(default = "default_tokens_per_file_warn")]
    pub tokens_per_file_warn: u64,
    #[serde(default = "default_tokens_per_turn_warn")]
    pub tokens_per_turn_warn: u64,
    #[serde(default = "default_max_session_hours")]
    pub max_session_hours: f64,
    #[serde(default)]
    pub solo_project: bool,
    #[serde(default)]
    pub attention: AttentionThresholds,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            contribution_concentration_warn: default_concentration_warn(),
            contribution_concentration_concern: default_concentration_concern(),
            ai_dependency_concern: default_ai_dependency_concern(),
            ai_dependency_info: default_ai_dependency_info(),
            tokens_per_file_warn: default_tokens_per_file_warn(),
            tokens_per_turn_warn: default_tokens_per_turn_warn(),
            max_session_hours: default_max_session_hours(),
            solo_project: false,
            attention: AttentionThresholds::default(),
        }
    }
}

fn default_concentration_warn() -> f64 { 0.80 }
fn default_concentration_concern() -> f64 { 0.60 }
fn default_ai_dependency_concern() -> f64 { 12.0 }
fn default_ai_dependency_info() -> f64 { 6.0 }
fn default_tokens_per_file_warn() -> u64 { 100_000 }
fn default_tokens_per_turn_warn() -> u64 { 20_000 }
fn default_max_session_hours() -> f64 { 12.0 }

impl Manifest {
    pub fn load(project_dir: &Path) -> Option<Self> {
        let path = Self::find_path(project_dir)?;
        let content = std::fs::read_to_string(&path).ok()?;
        let manifest: Self = serde_yaml::from_str(&content).ok()?;
        Some(manifest)
    }

    pub fn find_path(project_dir: &Path) -> Option<PathBuf> {
        let candidates = [
            project_dir.join("conscience.yaml"),
            project_dir.join("conscience.yml"),
            project_dir.join(".conscience.yaml"),
            project_dir.join(".conscience.yml"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }
}
