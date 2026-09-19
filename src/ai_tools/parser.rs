use crate::ai_tools::models::AiUsageSummary;
use crate::error::Result;
use crate::project::ProjectScope;

/// Trait that each AI tool parser implements.
///
/// The contract: produce a normalized AiUsageSummary from wherever the tool
/// stores its logs, optionally restricted to one resolved project.
/// Each implementation knows where its tool stores data and how to parse it.
pub trait AiToolParser {
    fn tool_name(&self) -> &str;

    /// Check whether this tool's logs exist at the expected location
    fn detect(&self) -> bool;

    /// Where this tool stores its data (for display purposes)
    fn data_path(&self) -> String;

    /// Parse logs and produce a summary. `None` means every project the
    /// tool has data for; `Some(scope)` restricts to that project's root
    /// and declared worktrees, matched exactly.
    fn parse(&self, scope: Option<&ProjectScope>) -> Result<AiUsageSummary>;
}
