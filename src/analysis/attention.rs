use crate::ai_tools::models::AiSession;
use crate::ethics::manifest::AttentionThresholds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// A moment of genuine human attention: one deduplicated prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Touchpoint {
    pub at: DateTime<Utc>,
    pub project: String,
    pub session_id: String,
    pub ai_until: Option<DateTime<Utc>>,
}

pub fn apply_alias(path: &str, aliases: &BTreeMap<String, String>) -> String {
    for (pattern, name) in aliases {
        if let Some(prefix) = pattern.strip_suffix('*') {
            if path.starts_with(prefix) {
                return name.clone();
            }
        } else if path == pattern {
            return name.clone();
        }
    }
    path.to_string()
}

pub fn collect_touchpoints(
    sessions: &[AiSession],
    th: &AttentionThresholds,
    since: DateTime<Utc>,
) -> Vec<Touchpoint> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut tps = Vec::new();

    for s in sessions {
        let project = apply_alias(
            s.project_path.as_deref().unwrap_or("(unknown)"),
            &th.project_aliases,
        );
        for i in &s.interactions {
            if i.human_at < since {
                continue;
            }
            let key = match &i.uuid {
                Some(u) => format!("uuid:{}", u),
                None => format!("ts:{}:{}", i.human_at.timestamp_millis(), project),
            };
            if !seen.insert(key) {
                continue;
            }
            tps.push(Touchpoint {
                at: i.human_at,
                project: project.clone(),
                session_id: s.session_id.clone(),
                ai_until: i.ai_until,
            });
        }
    }

    tps.sort_by_key(|t| t.at);
    tps
}
