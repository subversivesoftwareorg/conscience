use crate::ai_tools::models::AiSession;
use crate::analysis::energy;
use crate::ethics::manifest::EnergyConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRetro {
    pub session_count: u64,
    pub total_tokens: u64,
    pub total_output: u64,
    pub total_input: u64,
    pub total_cache_create: u64,
    pub total_cache_read: u64,
    pub cache_efficiency: f64,
    pub total_energy_wh: f64,
    pub per_session: Vec<SessionTokenProfile>,
    pub tool_summary: Vec<(String, u64)>,
    pub total_agent_dispatches: u64,
    pub total_skill_invocations: u64,
    pub diagnoses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokenProfile {
    pub session_id: String,
    pub project: String,
    pub model: String,
    pub duration_hours: f64,
    pub total_tokens: u64,
    pub output_tokens: u64,
    pub input_tokens: u64,
    pub cache_create: u64,
    pub cache_read: u64,
    pub input_pct: f64,
    pub output_pct: f64,
    pub cache_efficiency: f64,
    pub human_turns: u64,
    pub assistant_turns: u64,
    pub agent_count: u64,
    pub skill_count: u64,
    pub agents: Vec<String>,
    pub skills: Vec<String>,
    pub top_tools: Vec<(String, u64)>,
    pub energy_wh: f64,
}

pub fn analyze_token_retro(sessions: &[AiSession]) -> TokenRetro {
    if sessions.is_empty() {
        return TokenRetro {
            session_count: 0,
            total_tokens: 0,
            total_output: 0,
            total_input: 0,
            total_cache_create: 0,
            total_cache_read: 0,
            cache_efficiency: 0.0,
            total_energy_wh: 0.0,
            per_session: Vec::new(),
            tool_summary: Vec::new(),
            total_agent_dispatches: 0,
            total_skill_invocations: 0,
            diagnoses: Vec::new(),
        };
    }

    let energy_config = EnergyConfig::default();
    let energy_est = energy::estimate_total_energy(sessions, &energy_config);

    let mut profiles: Vec<SessionTokenProfile> = sessions
        .iter()
        .map(|s| {
            let total = s.tokens.input + s.tokens.output + s.tokens.cache_creation + s.tokens.cache_read;
            let cache_total = s.tokens.cache_creation + s.tokens.cache_read;
            let cache_eff = if cache_total > 0 {
                s.tokens.cache_read as f64 / cache_total as f64 * 100.0
            } else {
                0.0
            };

            let duration = match (s.started_at, s.ended_at) {
                (Some(start), Some(end)) => (end - start).num_minutes() as f64 / 60.0,
                _ => 0.0,
            };

            let coefficients = energy::resolve_coefficients(
                s.model.as_deref().unwrap_or("(unknown)"),
                &energy_config.overrides,
            );
            let se = energy::estimate_session_energy(s, &coefficients);

            let mut top_tools: Vec<(String, u64)> = s.tools_used.iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            top_tools.sort_by(|a, b| b.1.cmp(&a.1));

            SessionTokenProfile {
                session_id: s.session_id.clone(),
                project: s.project_path.clone().unwrap_or_default()
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown")
                    .to_string(),
                model: s.model.clone().unwrap_or_else(|| "(unknown)".to_string()),
                duration_hours: duration,
                total_tokens: total,
                output_tokens: s.tokens.output,
                input_tokens: s.tokens.input,
                cache_create: s.tokens.cache_creation,
                cache_read: s.tokens.cache_read,
                input_pct: if total > 0 { s.tokens.input as f64 / total as f64 * 100.0 } else { 0.0 },
                output_pct: if total > 0 { s.tokens.output as f64 / total as f64 * 100.0 } else { 0.0 },
                cache_efficiency: cache_eff,
                human_turns: s.turns.human,
                assistant_turns: s.turns.assistant,
                agent_count: s.agent_dispatches.len() as u64,
                skill_count: s.skill_invocations.len() as u64,
                agents: s.agent_dispatches.iter().map(|a| a.description.clone()).collect(),
                skills: s.skill_invocations.iter().map(|sk| sk.skill.clone()).collect(),
                top_tools,
                energy_wh: se.total_wh,
            }
        })
        .collect();

    profiles.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));

    let total_tokens: u64 = profiles.iter().map(|p| p.total_tokens).sum();
    let total_output: u64 = profiles.iter().map(|p| p.output_tokens).sum();
    let total_input: u64 = profiles.iter().map(|p| p.input_tokens).sum();
    let total_cache_create: u64 = profiles.iter().map(|p| p.cache_create).sum();
    let total_cache_read: u64 = profiles.iter().map(|p| p.cache_read).sum();
    let total_agents: u64 = profiles.iter().map(|p| p.agent_count).sum();
    let total_skills: u64 = profiles.iter().map(|p| p.skill_count).sum();

    let cache_total = total_cache_create + total_cache_read;
    let cache_efficiency = if cache_total > 0 {
        total_cache_read as f64 / cache_total as f64 * 100.0
    } else {
        0.0
    };

    // Aggregate tool usage
    let mut tool_totals: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for s in sessions {
        for (tool, count) in &s.tools_used {
            *tool_totals.entry(tool.clone()).or_insert(0) += count;
        }
    }
    let mut tool_summary: Vec<(String, u64)> = tool_totals.into_iter().collect();
    tool_summary.sort_by(|a, b| b.1.cmp(&a.1));

    // Diagnoses
    let mut diagnoses = Vec::new();

    if total_tokens > 0 && total_input as f64 / total_tokens as f64 > 0.6 {
        diagnoses.push(format!(
            "Input tokens dominated at {:.0}% of total — context was growing faster than output. \
            Long conversations or large file reads accumulate input tokens each turn.",
            total_input as f64 / total_tokens as f64 * 100.0
        ));
    }

    if cache_total > 0 && cache_efficiency < 30.0 {
        diagnoses.push(format!(
            "Cache efficiency was only {:.0}% — context was being rebuilt frequently. \
            Session restarts, compaction, or model switches cause cache rebuilds.",
            cache_efficiency
        ));
    }

    if total_agents > 5 {
        let heaviest = profiles.iter().max_by_key(|p| p.agent_count).unwrap();
        diagnoses.push(format!(
            "{} agent dispatches across {} sessions. \
            Session \"{}\" dispatched {} agents. Each dispatch grows the main context \
            with the prompt and result, compounding input token costs.",
            total_agents,
            profiles.iter().filter(|p| p.agent_count > 0).count(),
            heaviest.session_id.chars().take(8).collect::<String>(),
            heaviest.agent_count,
        ));
    }

    let total_reads = tool_summary.iter().find(|(t, _)| t == "Read").map(|(_, c)| *c).unwrap_or(0);
    let total_unique_files: std::collections::HashSet<&str> = sessions.iter()
        .flat_map(|s| s.files_touched.iter().map(|f| f.path.as_str()))
        .collect();
    if total_reads > 20 && total_unique_files.len() > 0 && total_reads as f64 / total_unique_files.len() as f64 > 3.0 {
        diagnoses.push(format!(
            "{} Read calls on {} unique files ({:.1}x re-read ratio). \
            Re-reading files inflates input tokens. Consider whether context is being lost \
            and rebuilt between turns.",
            total_reads,
            total_unique_files.len(),
            total_reads as f64 / total_unique_files.len() as f64,
        ));
    }

    if profiles.len() > 1 {
        let top = &profiles[0];
        let top_share = top.total_tokens as f64 / total_tokens as f64 * 100.0;
        if top_share > 50.0 {
            diagnoses.push(format!(
                "Session \"{}\" ({}) consumed {:.0}% of all tokens. \
                The budget drain was concentrated in this session.",
                top.session_id.chars().take(8).collect::<String>(),
                top.project,
                top_share,
            ));
        }
    }

    TokenRetro {
        session_count: profiles.len() as u64,
        total_tokens,
        total_output,
        total_input,
        total_cache_create,
        total_cache_read,
        cache_efficiency,
        total_energy_wh: energy_est.total_wh,
        per_session: profiles,
        tool_summary,
        total_agent_dispatches: total_agents,
        total_skill_invocations: total_skills,
        diagnoses,
    }
}
