use conscience::ai_tools::models::*;
use conscience::analysis::energy::*;
use conscience::ethics::manifest::EnergyConfig;
use conscience::ethics::manifest::EnergyOverride;
use std::collections::BTreeMap;

fn session_with(model: &str, input: u64, output: u64, cache_create: u64, cache_read: u64) -> AiSession {
    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: "test".to_string(),
        project_path: None,
        started_at: None,
        ended_at: None,
        model: Some(model.to_string()),
        work_categories: vec![],
        turns: TurnCounts::default(),
        tokens: TokenUsage { input, output, cache_creation: cache_create, cache_read },
        tools_used: Default::default(),
        files_touched: vec![],
        bash_commands: vec![],
        agent_actions: vec![],
        git_branch: None,
        interactions: vec![],
    }
}

#[test]
fn large_tier_coefficients_for_known_model() {
    let c = resolve_coefficients("claude-fable-5", &BTreeMap::new());
    assert_eq!(c.tier, "large");
    assert_eq!(c.wh_per_1k_output, 1.5);
    assert_eq!(c.wh_per_1k_input, 0.3);
    assert_eq!(c.uncertainty_pct, 50.0);
}

#[test]
fn frontier_tier_for_opus() {
    let c = resolve_coefficients("claude-opus-5", &BTreeMap::new());
    assert_eq!(c.tier, "frontier");
    assert_eq!(c.wh_per_1k_output, 3.0);
}

#[test]
fn unknown_model_falls_to_unknown_tier() {
    let c = resolve_coefficients("some-future-model-99", &BTreeMap::new());
    assert_eq!(c.tier, "unknown");
    assert_eq!(c.wh_per_1k_output, 1.0);
    assert_eq!(c.uncertainty_pct, 200.0);
}

#[test]
fn override_replaces_tier_values() {
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "claude-fable-5".to_string(),
        EnergyOverride {
            wh_per_1k_input: Some(0.5),
            wh_per_1k_output: Some(2.0),
        },
    );
    let c = resolve_coefficients("claude-fable-5", &overrides);
    assert_eq!(c.wh_per_1k_input, 0.5);
    assert_eq!(c.wh_per_1k_output, 2.0);
    assert!((c.wh_per_1k_cache_read - 0.05).abs() < 0.001);
}

#[test]
fn session_energy_arithmetic() {
    let s = session_with("claude-fable-5", 0, 1000, 0, 0);
    let c = resolve_coefficients("claude-fable-5", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert!((e.total_wh - 1.5).abs() < 0.001);
    assert!((e.output_wh - 1.5).abs() < 0.001);
    assert_eq!(e.input_wh, 0.0);
}

#[test]
fn session_energy_all_token_types() {
    let s = session_with("claude-fable-5", 10_000, 10_000, 10_000, 10_000);
    let c = resolve_coefficients("claude-fable-5", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert!((e.total_wh - 22.05).abs() < 0.01);
}

#[test]
fn total_energy_with_mixed_models() {
    let sessions = vec![
        session_with("claude-fable-5", 0, 1000, 0, 0),
        session_with("claude-opus-5", 0, 1000, 0, 0),
        session_with("claude-haiku-4-5", 0, 1000, 0, 0),
    ];
    let config = EnergyConfig::default();
    let est = estimate_total_energy(&sessions, &config);
    assert!((est.total_wh - 5.0).abs() < 0.01);
    assert_eq!(est.per_model.len(), 3);
    assert!(est.uncertainty_range.0 < est.total_wh);
    assert!(est.uncertainty_range.1 > est.total_wh);
}

#[test]
fn zero_tokens_yields_zero_energy() {
    let s = session_with("claude-fable-5", 0, 0, 0, 0);
    let c = resolve_coefficients("claude-fable-5", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert_eq!(e.total_wh, 0.0);
}

#[test]
fn co2_estimate_with_grid_intensity() {
    let sessions = vec![session_with("claude-fable-5", 0, 10_000, 0, 0)];
    let config = EnergyConfig {
        grid_carbon_intensity: Some(0.42),
        ..Default::default()
    };
    let est = estimate_total_energy(&sessions, &config);
    assert!((est.co2_kg.unwrap() - 0.0063).abs() < 0.001);
}

#[test]
fn no_model_uses_unknown_tier() {
    let mut s = session_with("claude-fable-5", 0, 1000, 0, 0);
    s.model = None;
    let c = resolve_coefficients("(unknown)", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert!((e.total_wh - 1.0).abs() < 0.01);
}
