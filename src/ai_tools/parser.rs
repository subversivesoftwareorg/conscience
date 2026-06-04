use crate::ai_tools::models::AiUsageSummary;
use crate::error::Result;
use std::path::Path;

/// Trait that each AI tool parser implements.
///
/// The contract: given a path to scan (could be a directory of logs,
/// a config directory, etc.), produce a normalized AiUsageSummary.
/// Each implementation knows where its tool stores data and how to parse it.
pub trait AiToolParser {
    fn tool_name(&self) -> &str;

    /// Check whether this tool's logs exist at the expected location
    fn detect(&self) -> bool;

    /// Where this tool stores its data (for display purposes)
    fn data_path(&self) -> String;

    /// Parse logs and produce a summary, optionally filtered to a project path
    fn parse(&self, project_filter: Option<&Path>) -> Result<AiUsageSummary>;
}
