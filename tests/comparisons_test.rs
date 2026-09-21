//! Everyday comparisons must be chosen by scale, name their sources,
//! honour team overrides, carry uncertainty, and never leave a non-zero
//! estimate without at least one.

use conscience::analysis::comparisons::*;
use conscience::ethics::manifest::{ComparisonOverride, EnergyConfig};
use std::collections::BTreeMap;

fn keys(list: &[Comparison]) -> Vec<&str> {
    list.iter().map(|c| c.key.as_str()).collect()
}

#[test]
fn a_session_reads_in_phone_charges_and_laptop_hours() {
    let refs = default_references();
    // ~90 Wh: one long session.
    let picked = select(90.0, 45.0, 135.0, Quantity::Energy, &refs, 2);
    assert_eq!(keys(&picked), ["phone_charge", "laptop_hour"]);
    assert!((picked[0].count - 6.0).abs() < 1e-9);
    assert!((picked[0].low - 3.0).abs() < 1e-9);
    assert!((picked[0].high - 9.0).abs() < 1e-9);
    assert_eq!(picked[0].phrase(), "6 full phone charges (3\u{2013}9)");
}

#[test]
fn a_week_across_projects_reads_in_households_and_miles() {
    let refs = default_references();
    // 21 kWh: the real figure from a 7-day cross-project run.
    let picked = select(21_000.0, 10_500.0, 31_500.0, Quantity::Energy, &refs, 2);
    // Both ratios sit near the ideal "about ten of something".
    assert!(
        picked
            .iter()
            .all(|c| ["ev_mile", "eu_household_day", "us_household_day"].contains(&c.key.as_str())),
        "{:?}",
        keys(&picked)
    );
    assert!(
        !keys(&picked).contains(&"phone_charge"),
        "1400 phone charges is not legible"
    );
}

#[test]
fn a_month_of_heavy_use_prefers_household_days() {
    let refs = default_references();
    let picked = select(300_000.0, 150_000.0, 450_000.0, Quantity::Energy, &refs, 2);
    assert!(
        keys(&picked).contains(&"us_household_day"),
        "{:?}",
        keys(&picked)
    );
    assert!(
        !keys(&picked).contains(&"laptop_hour"),
        "5000 laptop hours is not legible"
    );
}

#[test]
fn co2_and_water_have_their_own_references() {
    let refs = default_references();
    let co2 = select(8.8, 4.4, 13.2, Quantity::Co2, &refs, 2);
    assert!(keys(&co2).contains(&"car_mile"), "{:?}", keys(&co2));
    assert!(co2.iter().all(|c| c.quantity == Quantity::Co2));

    let water = select(38.0, 19.0, 57.0, Quantity::Water, &refs, 2);
    assert!(water.iter().all(|c| c.quantity == Quantity::Water));
    assert!(!water.is_empty());
}

#[test]
fn nonzero_estimate_always_gets_one_comparison_and_zero_gets_none() {
    let refs = default_references();
    // 0.5 Wh is below the readable band for everything; nearest wins.
    let tiny = select(0.5, 0.25, 0.75, Quantity::Energy, &refs, 2);
    assert_eq!(tiny.len(), 1);
    assert_eq!(tiny[0].key, "phone_charge");

    assert!(select(0.0, 0.0, 0.0, Quantity::Energy, &refs, 2).is_empty());
}

#[test]
fn every_default_reference_names_a_source_and_positive_value() {
    for r in default_references() {
        assert!(r.value > 0.0, "{}", r.key);
        assert!(!r.source.is_empty(), "{} has no source", r.key);
        assert!(!r.singular.is_empty() && !r.plural.is_empty(), "{}", r.key);
    }
}

#[test]
fn overrides_adjust_disable_and_add() {
    let mut overrides: BTreeMap<String, ComparisonOverride> = BTreeMap::new();
    // A German team: smaller household figure, own source.
    overrides.insert(
        "eu_household_day".into(),
        ComparisonOverride {
            value: Some(8_500.0),
            source: Some("Destatis 2023".into()),
            ..Default::default()
        },
    );
    overrides.insert(
        "us_household_day".into(),
        ComparisonOverride {
            disabled: Some(true),
            ..Default::default()
        },
    );
    overrides.insert(
        "train_km".into(),
        ComparisonOverride {
            quantity: Some(Quantity::Energy),
            value: Some(100.0),
            plural: Some("km of electric-train travel".into()),
            source: Some("team estimate".into()),
            ..Default::default()
        },
    );

    let refs = references_with(&overrides);
    let eu = refs.iter().find(|r| r.key == "eu_household_day").unwrap();
    assert_eq!(eu.value, 8_500.0);
    assert_eq!(eu.source, "Destatis 2023");
    assert!(
        refs.iter().all(|r| r.key != "us_household_day"),
        "disabled entries are gone"
    );
    let train = refs.iter().find(|r| r.key == "train_km").unwrap();
    assert_eq!(
        train.singular, "km of electric-train travel",
        "singular falls back to plural"
    );

    // An added entry without the required fields is ignored, not a crash.
    let mut bad: BTreeMap<String, ComparisonOverride> = BTreeMap::new();
    bad.insert(
        "half_baked".into(),
        ComparisonOverride {
            value: Some(1.0),
            ..Default::default()
        },
    );
    assert!(references_with(&bad).iter().all(|r| r.key != "half_baked"));
}

#[test]
fn for_estimate_scales_co2_and_water_uncertainty_with_energy() {
    let config = EnergyConfig::default();
    let c = Comparisons::for_estimate(2_000.0, (1_000.0, 3_000.0), Some(0.84), Some(3.6), &config);
    assert!(!c.energy.is_empty());
    let co2 = &c.co2[0];
    assert!((co2.low / co2.count - 0.5).abs() < 1e-9);
    assert!((co2.high / co2.count - 1.5).abs() < 1e-9);
    let water = &c.water[0];
    assert!((water.low / water.count - 0.5).abs() < 1e-9);

    // Caveats travel with the numbers.
    assert!(c.notes.iter().any(|n| n.contains("illustrative")));
    assert!(c.notes.iter().any(|n| n.contains("location-based")));
    assert!(c.notes.iter().any(|n| n.contains("Li et al. 2023")));

    // No intensity configured means no CO2 comparisons and no CO2 note.
    let no_co2 = EnergyConfig {
        grid_carbon_intensity: None,
        ..Default::default()
    };
    let c = Comparisons::for_estimate(2_000.0, (1_000.0, 3_000.0), None, Some(3.6), &no_co2);
    assert!(c.co2.is_empty());
    assert!(!c.notes.iter().any(|n| n.contains("location-based")));
}

#[test]
fn per_day_divides_by_the_interval_and_never_by_zero() {
    let d = per_day(7, 21_000.0, Some(8.82), Some(37.8));
    assert_eq!(d.days, 7);
    assert!((d.wh - 3_000.0).abs() < 1e-9);
    assert!((d.co2_kg.unwrap() - 1.26).abs() < 1e-9);
    assert!((d.water_liters.unwrap() - 5.4).abs() < 1e-9);
    let z = per_day(0, 100.0, None, None);
    assert_eq!(z.days, 1);
    assert_eq!(z.wh, 100.0);
}

#[test]
fn energy_estimate_carries_comparisons_into_json() {
    use conscience::ai_tools::models::*;
    let session = AiSession {
        tool: AiTool::ClaudeCode,
        session_id: "s".into(),
        project_path: None,
        started_at: None,
        ended_at: None,
        model: Some("claude-sonnet-4".into()),
        work_categories: vec![],
        turns: TurnCounts::default(),
        tokens: TokenUsage {
            input: 0,
            output: 60_000,
            cache_creation: 0,
            cache_read: 0,
        },
        tools_used: Default::default(),
        files_touched: vec![],
        bash_commands: vec![],
        agent_actions: vec![],
        git_branch: None,
        interactions: vec![],
        agent_dispatches: vec![],
        skill_invocations: vec![],
        launch: Default::default(),
    };
    let est =
        conscience::analysis::energy::estimate_total_energy(&[session], &EnergyConfig::default());
    assert!((est.total_wh - 90.0).abs() < 1e-9);
    assert_eq!(est.comparisons.energy[0].key, "phone_charge");
    let json = serde_json::to_value(&est).unwrap();
    assert!(json["comparisons"]["energy"][0]["source"].is_string());
    assert!(json["comparisons"]["notes"].is_array());
}
