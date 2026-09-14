use crate::ai_tools::models::*;
use crate::ai_tools::parser::AiToolParser;
use crate::error::{ConscienceError, Result};
use std::path::{Path, PathBuf};

pub struct WindsurfParser;

impl WindsurfParser {
    fn log_dirs() -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            home.join(".windsurf"),
            home.join(".codeium"),
            home.join("Library/Application Support/Windsurf"),
            home.join(".config/Windsurf"),
        ]
    }

    pub fn detected_path() -> Option<PathBuf> {
        Self::log_dirs().into_iter().find(|p| p.exists())
    }
}

impl AiToolParser for WindsurfParser {
    fn tool_name(&self) -> &str {
        "Windsurf"
    }

    fn detect(&self) -> bool {
        Self::detected_path().is_some()
    }

    fn data_path(&self) -> String {
        Self::detected_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "(not found)".to_string())
    }

    fn parse(&self, _project_filter: Option<&Path>) -> Result<AiUsageSummary> {
        Err(ConscienceError::Other(anyhow::anyhow!(
            "Windsurf parser is not yet implemented.\n\
            Help us build it! If you use Windsurf, please share sample log data at:\n\
            https://github.com/subversivesoftwareorg/roadmap/issues/74"
        )))
    }
}
