use crate::ai_tools::models::*;
use crate::ai_tools::parser::AiToolParser;
use crate::error::{ConscienceError, Result};
use crate::project::ProjectScope;
use std::path::PathBuf;

pub struct CursorParser;

impl CursorParser {
    fn log_dirs() -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            home.join(".cursor"),
            home.join("Library/Application Support/Cursor"),
            home.join("Library/Application Support/Cursor/User/globalStorage"),
            home.join(".config/Cursor"),
        ]
    }

    pub fn detected_path() -> Option<PathBuf> {
        Self::log_dirs().into_iter().find(|p| p.exists())
    }
}

impl AiToolParser for CursorParser {
    fn tool_name(&self) -> &str {
        "Cursor"
    }

    fn detect(&self) -> bool {
        Self::detected_path().is_some()
    }

    fn data_path(&self) -> String {
        Self::detected_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "(not found)".to_string())
    }

    fn parse(&self, _scope: Option<&ProjectScope>) -> Result<AiUsageSummary> {
        Err(ConscienceError::Other(anyhow::anyhow!(
            "Cursor parser is not yet implemented.\n\
            Help us build it! If you use Cursor, please share sample log data at:\n\
            https://github.com/subversivesoftwareorg/roadmap/issues/72"
        )))
    }
}
