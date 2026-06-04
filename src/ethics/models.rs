use serde::{Deserialize, Serialize};

/// The seven ethical principles Conscience evaluates, drawn from the reference documents.
/// Each principle maps to concrete signals that can be detected from data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Principle {
    HumanAgency,
    EquityOfBenefit,
    Transparency,
    DeveloperGrowth,
    EnvironmentalCost,
    CodeProvenance,
    Security,
}

impl Principle {
    pub fn all() -> &'static [Principle] {
        &[
            Principle::HumanAgency,
            Principle::EquityOfBenefit,
            Principle::Transparency,
            Principle::DeveloperGrowth,
            Principle::EnvironmentalCost,
            Principle::CodeProvenance,
            Principle::Security,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            Principle::HumanAgency => "Human Agency",
            Principle::EquityOfBenefit => "Equity of Benefit",
            Principle::Transparency => "Transparency",
            Principle::DeveloperGrowth => "Developer Growth",
            Principle::EnvironmentalCost => "Environmental Cost",
            Principle::CodeProvenance => "Code Provenance",
            Principle::Security => "Security",
        }
    }

    pub fn source(&self) -> &str {
        match self {
            Principle::HumanAgency => "MH 150: technology can \"de-skill workers, subject them to automated surveillance\"",
            Principle::EquityOfBenefit => "MH 73: \"no one is saved alone\" / MH 77: institutions must serve all persons",
            Principle::Transparency => "Leiden O1: \"Transparently disclose the use of automated tools\"",
            Principle::DeveloperGrowth => "MH 52: human value does not depend on output; MH 129: does AI make life \"more human\"?",
            Principle::EnvironmentalCost => "MH 101: AI systems \"require enormous amounts of energy and water\"",
            Principle::CodeProvenance => "Leiden O4-O6: retain responsibility, affirm human authorship, proper attribution",
            Principle::Security => "MH 104: \"we cannot consider AI to be morally neutral\"; Leiden O4: retain responsibility for correctness",
        }
    }

    pub fn question(&self) -> &str {
        match self {
            Principle::HumanAgency => "Are humans directing the work, or becoming dependent on AI to function?",
            Principle::EquityOfBenefit => "Are AI tools benefiting all team members, or concentrating advantage?",
            Principle::Transparency => "Is AI involvement disclosed? Can stakeholders see what was human vs. machine?",
            Principle::DeveloperGrowth => "Are team members learning and growing, or being de-skilled?",
            Principle::EnvironmentalCost => "Is AI usage proportionate to the value delivered?",
            Principle::CodeProvenance => "Do humans understand and take responsibility for AI-generated code?",
            Principle::Security => "Are AI tools being used safely? Are there signs of misuse, data exposure, or unattended automation?",
        }
    }
}

/// Per-project analysis result for multi-project reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    pub project_path: String,
    pub project_name: Option<String>,
    pub analysis: EthicalAnalysis,
    pub session_count: u64,
    pub total_output_tokens: u64,
    pub ai_human_ratio: f64,
}

/// Aggregated analysis across multiple projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiProjectAnalysis {
    pub projects: Vec<ProjectAnalysis>,
    pub outlier_signals: Vec<Signal>,
    pub total_projects: usize,
    pub total_sessions: u64,
    pub total_output_tokens: u64,
}

/// How severe or noteworthy a signal is.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Healthy,
    Info,
    Concern,
    Warning,
}

impl Severity {
    pub fn symbol(&self) -> &str {
        match self {
            Severity::Healthy => "+",
            Severity::Info => "?",
            Severity::Concern => "!",
            Severity::Warning => "!!",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Healthy => write!(f, "HEALTHY"),
            Severity::Info => write!(f, "INFO"),
            Severity::Concern => write!(f, "CONCERN"),
            Severity::Warning => write!(f, "WARNING"),
        }
    }
}

/// A concrete observation from the data that may have ethical implications.
/// Signals never render verdicts — they present evidence for human judgment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub principle: Principle,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub evidence: String,
}

/// A dimension score on the scorecard. Some are automated, some need human input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub principle: Principle,
    pub auto_signals: Vec<Signal>,
    pub needs_human_input: bool,
    pub human_assessment: Option<String>,
}

/// A question for team reflection, populated with real data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionQuestion {
    pub principle: Principle,
    pub data_context: String,
    pub question: String,
}

/// The full ethical analysis output, combining all three layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalAnalysis {
    pub signals: Vec<Signal>,
    pub scorecard: Vec<DimensionScore>,
    pub reflections: Vec<ReflectionQuestion>,
}
