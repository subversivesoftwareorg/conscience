use crate::ai_tools::models::{AiSession, TokenUsage};
use crate::ethics::manifest::{EnergyConfig, EnergyOverride};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyCoefficients {
    pub wh_per_1k_input: f64,
    pub wh_per_1k_output: f64,
    pub wh_per_1k_cache_create: f64,
    pub wh_per_1k_cache_read: f64,
    pub uncertainty_pct: f64,
    pub tier: String,
    pub source_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnergy {
    pub session_id: String,
    pub model: String,
    pub tier: String,
    pub input_wh: f64,
    pub output_wh: f64,
    pub cache_create_wh: f64,
    pub cache_read_wh: f64,
    pub total_wh: f64,
    pub uncertainty_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEnergy {
    pub model: String,
    pub tier: String,
    pub sessions: u64,
    pub total_wh: f64,
    pub uncertainty_pct: f64,
    pub tokens: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyEstimate {
    pub period_days: u32,
    pub per_model: Vec<ModelEnergy>,
    pub per_session: Vec<SessionEnergy>,
    pub total_wh: f64,
    pub total_kwh: f64,
    pub blended_wh_per_1k_output: f64,
    pub uncertainty_range: (f64, f64),
    pub co2_kg: Option<f64>,
    pub grid_carbon_intensity: Option<f64>,
    pub methodology: String,
}

struct TierDef {
    prefix: &'static str,
    tier: &'static str,
    wh_per_1k_output: f64,
    wh_per_1k_input: f64,
    uncertainty_pct: f64,
    source_note: &'static str,
}

fn tier_table() -> Vec<TierDef> {
    vec![
        TierDef { prefix: "claude-opus", tier: "frontier", wh_per_1k_output: 3.0, wh_per_1k_input: 0.60, uncertainty_pct: 100.0, source_note: "Jegham 2025 (o3: 7.03 Wh/short)" },
        TierDef { prefix: "claude-fable", tier: "large", wh_per_1k_output: 1.5, wh_per_1k_input: 0.30, uncertainty_pct: 50.0, source_note: "mdodkins 2026, Jegham 2025 (Sonnet 3.7: 0.84 Wh/short)" },
        TierDef { prefix: "claude-sonnet", tier: "large", wh_per_1k_output: 1.5, wh_per_1k_input: 0.30, uncertainty_pct: 50.0, source_note: "mdodkins 2026, Jegham 2025" },
        TierDef { prefix: "claude-haiku", tier: "medium", wh_per_1k_output: 0.5, wh_per_1k_input: 0.10, uncertainty_pct: 50.0, source_note: "Jegham 2025 (4o-mini: 0.42 Wh/short)" },
        TierDef { prefix: "gpt-4.1-nano", tier: "small", wh_per_1k_output: 0.15, wh_per_1k_input: 0.03, uncertainty_pct: 50.0, source_note: "Jegham 2025 (nano: 0.10 Wh/short)" },
        TierDef { prefix: "gpt-4o-mini", tier: "medium", wh_per_1k_output: 0.5, wh_per_1k_input: 0.10, uncertainty_pct: 50.0, source_note: "Jegham 2025" },
        TierDef { prefix: "gpt-4.1-mini", tier: "medium", wh_per_1k_output: 0.5, wh_per_1k_input: 0.10, uncertainty_pct: 50.0, source_note: "Jegham 2025" },
        TierDef { prefix: "gemini-flash", tier: "small", wh_per_1k_output: 0.15, wh_per_1k_input: 0.03, uncertainty_pct: 50.0, source_note: "IEA 2025 (Gemini: 0.24 Wh/query)" },
        TierDef { prefix: "gpt-4", tier: "large", wh_per_1k_output: 1.5, wh_per_1k_input: 0.30, uncertainty_pct: 50.0, source_note: "Jegham 2025" },
        TierDef { prefix: "o3", tier: "reasoning", wh_per_1k_output: 5.0, wh_per_1k_input: 0.60, uncertainty_pct: 150.0, source_note: "Jegham 2025 (o3: 7.03 Wh/short)" },
        TierDef { prefix: "o1", tier: "reasoning", wh_per_1k_output: 5.0, wh_per_1k_input: 0.60, uncertainty_pct: 150.0, source_note: "Jegham 2025 (o1: 4.45 Wh/short)" },
    ]
}

fn build_coefficients(
    wh_per_1k_input: f64,
    wh_per_1k_output: f64,
    uncertainty_pct: f64,
    tier: &str,
    source_note: &str,
) -> EnergyCoefficients {
    EnergyCoefficients {
        wh_per_1k_input,
        wh_per_1k_output,
        wh_per_1k_cache_create: wh_per_1k_input * 1.25,
        wh_per_1k_cache_read: wh_per_1k_input * 0.1,
        uncertainty_pct,
        tier: tier.to_string(),
        source_note: source_note.to_string(),
    }
}

pub fn resolve_coefficients(
    model: &str,
    overrides: &BTreeMap<String, EnergyOverride>,
) -> EnergyCoefficients {
    if let Some(ov) = overrides.get(model) {
        for def in tier_table() {
            if model.starts_with(def.prefix) {
                let input = ov.wh_per_1k_input.unwrap_or(def.wh_per_1k_input);
                let output = ov.wh_per_1k_output.unwrap_or(def.wh_per_1k_output);
                return build_coefficients(input, output, def.uncertainty_pct, def.tier, def.source_note);
            }
        }
        let input = ov.wh_per_1k_input.unwrap_or(0.20);
        let output = ov.wh_per_1k_output.unwrap_or(1.0);
        return build_coefficients(input, output, 200.0, "unknown", "User override");
    }

    for def in tier_table() {
        if model.starts_with(def.prefix) {
            return build_coefficients(
                def.wh_per_1k_input,
                def.wh_per_1k_output,
                def.uncertainty_pct,
                def.tier,
                def.source_note,
            );
        }
    }

    build_coefficients(0.20, 1.0, 200.0, "unknown", "Blended estimate (model not in coefficient table)")
}

pub fn estimate_session_energy(session: &AiSession, coefficients: &EnergyCoefficients) -> SessionEnergy {
    let t = &session.tokens;
    let input_wh = t.input as f64 * coefficients.wh_per_1k_input / 1000.0;
    let output_wh = t.output as f64 * coefficients.wh_per_1k_output / 1000.0;
    let cache_create_wh = t.cache_creation as f64 * coefficients.wh_per_1k_cache_create / 1000.0;
    let cache_read_wh = t.cache_read as f64 * coefficients.wh_per_1k_cache_read / 1000.0;

    SessionEnergy {
        session_id: session.session_id.clone(),
        model: session.model.clone().unwrap_or_else(|| "(unknown)".to_string()),
        tier: coefficients.tier.clone(),
        input_wh,
        output_wh,
        cache_create_wh,
        cache_read_wh,
        total_wh: input_wh + output_wh + cache_create_wh + cache_read_wh,
        uncertainty_pct: coefficients.uncertainty_pct,
    }
}

pub fn estimate_total_energy(sessions: &[AiSession], config: &EnergyConfig) -> EnergyEstimate {
    let mut per_session = Vec::new();
    let mut by_model: BTreeMap<String, (f64, f64, u64, TokenUsage, String)> = BTreeMap::new();
    let mut total_wh = 0.0;
    let mut total_output_tokens = 0u64;
    let mut weighted_uncertainty_sum = 0.0;

    for session in sessions {
        let model_name = session.model.clone().unwrap_or_else(|| "(unknown)".to_string());
        let coefficients = resolve_coefficients(&model_name, &config.overrides);
        let se = estimate_session_energy(session, &coefficients);

        total_wh += se.total_wh;
        total_output_tokens += session.tokens.output;
        weighted_uncertainty_sum += se.total_wh * se.uncertainty_pct;

        let entry = by_model
            .entry(model_name.clone())
            .or_insert((0.0, coefficients.uncertainty_pct, 0, TokenUsage::default(), coefficients.tier.clone()));
        entry.0 += se.total_wh;
        entry.2 += 1;
        entry.3.input += session.tokens.input;
        entry.3.output += session.tokens.output;
        entry.3.cache_creation += session.tokens.cache_creation;
        entry.3.cache_read += session.tokens.cache_read;

        per_session.push(se);
    }

    let blended_uncertainty = if total_wh > 0.0 {
        weighted_uncertainty_sum / total_wh
    } else {
        0.0
    };

    let per_model: Vec<ModelEnergy> = by_model
        .into_iter()
        .map(|(model, (wh, unc, sess, tokens, tier))| ModelEnergy {
            model,
            tier,
            sessions: sess,
            total_wh: wh,
            uncertainty_pct: unc,
            tokens,
        })
        .collect();

    let blended_wh_per_1k_output = if total_output_tokens > 0 {
        total_wh / (total_output_tokens as f64 / 1000.0)
    } else {
        0.0
    };

    let co2_kg = config.grid_carbon_intensity.map(|gi| total_wh / 1000.0 * gi);

    EnergyEstimate {
        period_days: 0,
        per_model,
        per_session,
        total_wh,
        total_kwh: total_wh / 1000.0,
        blended_wh_per_1k_output,
        uncertainty_range: (
            total_wh * (1.0 - blended_uncertainty / 100.0),
            total_wh * (1.0 + blended_uncertainty / 100.0),
        ),
        co2_kg,
        grid_carbon_intensity: config.grid_carbon_intensity,
        methodology: "Estimates based on Jegham et al. 2025, mdodkins 2026, Patterson et al. 2025. \
            No provider publishes official per-model energy data.".to_string(),
    }
}
