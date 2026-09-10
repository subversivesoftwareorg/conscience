# Energy Accounting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Map token usage to estimated energy (Wh) per model, with `conscience report energy`, signal enrichment, and reflection enrichment.

**Architecture:** A coefficient table maps model-name prefixes to energy tiers. A pure `analysis/energy.rs` module estimates per-session and total Wh from `&[AiSession]`. The existing `report` subcommand gains an `Energy` variant. Existing EnvironmentalCost signal and proportionality reflection get energy context injected.

**Tech Stack:** Rust, existing deps only (serde, comfy-table, chrono).

**Spec:** `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

## Global Constraints

- All energy figures labeled "estimated" — never "consumed" or "used."
- Uncertainty ranges shown alongside every Wh figure.
- Methodology string cites Jegham et al. 2025, mdodkins 2026, Patterson et al. 2025.
- No new dependencies. Keep NEW code rustfmt-clean; don't reformat existing code.
- TDD: failing test first.
- All structs derive `Debug, Clone, Serialize, Deserialize`.

---

### Task 1: Energy module — coefficients, estimation, config

**Files:**
- Create: `src/analysis/energy.rs`
- Modify: `src/analysis/mod.rs` (add `pub mod energy;`)
- Modify: `src/ethics/manifest.rs` (add `EnergyConfig` + `EnergyOverride` under `Thresholds`)
- Create: `tests/energy_test.rs`

**Interfaces:**
- Consumes: `AiSession` (fields: `model: Option<String>`, `tokens: TokenUsage`, `session_id: String`), `TokenUsage { input, output, cache_creation, cache_read }`.
- Produces: `EnergyCoefficients`, `SessionEnergy`, `ModelEnergy`, `EnergyEstimate`, `EnergyConfig`, `EnergyOverride`, `resolve_coefficients()`, `estimate_session_energy()`, `estimate_total_energy()`.

- [ ] **Step 1: Write the failing tests** — create `tests/energy_test.rs`:

```rust
use conscience::ai_tools::models::*;
use conscience::analysis::energy::*;
use conscience::ethics::manifest::EnergyConfig;
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
    overrides.insert("claude-fable-5".to_string(), EnergyOverride {
        wh_per_1k_input: Some(0.5),
        wh_per_1k_output: Some(2.0),
    });
    let c = resolve_coefficients("claude-fable-5", &overrides);
    assert_eq!(c.wh_per_1k_input, 0.5);
    assert_eq!(c.wh_per_1k_output, 2.0);
    // cache coefficients still derived from overridden input
    assert!((c.wh_per_1k_cache_read - 0.05).abs() < 0.001);
}

#[test]
fn session_energy_arithmetic() {
    // 1000 output tokens of Large tier = 1.5 Wh
    let s = session_with("claude-fable-5", 0, 1000, 0, 0);
    let c = resolve_coefficients("claude-fable-5", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert!((e.total_wh - 1.5).abs() < 0.001);
    assert!((e.output_wh - 1.5).abs() < 0.001);
    assert_eq!(e.input_wh, 0.0);
}

#[test]
fn session_energy_all_token_types() {
    // 10K each of input, output, cache_create, cache_read at Large tier
    // input: 10 * 0.3 = 3.0, output: 10 * 1.5 = 15.0,
    // cache_create: 10 * 0.375 = 3.75, cache_read: 10 * 0.03 = 0.3
    // total = 22.05
    let s = session_with("claude-fable-5", 10_000, 10_000, 10_000, 10_000);
    let c = resolve_coefficients("claude-fable-5", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert!((e.total_wh - 22.05).abs() < 0.01);
}

#[test]
fn total_energy_with_mixed_models() {
    let sessions = vec![
        session_with("claude-fable-5", 0, 1000, 0, 0),     // 1.5 Wh
        session_with("claude-opus-5", 0, 1000, 0, 0),       // 3.0 Wh
        session_with("claude-haiku-4-5", 0, 1000, 0, 0),    // 0.5 Wh
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
    let sessions = vec![session_with("claude-fable-5", 0, 10_000, 0, 0)]; // 15 Wh
    let config = EnergyConfig {
        grid_carbon_intensity: Some(0.42),
        ..Default::default()
    };
    let est = estimate_total_energy(&sessions, &config);
    // 15 Wh = 0.015 kWh * 0.42 = 0.0063 kg
    assert!((est.co2_kg.unwrap() - 0.0063).abs() < 0.001);
}

#[test]
fn no_model_uses_unknown_tier() {
    let mut s = session_with("claude-fable-5", 0, 1000, 0, 0);
    s.model = None;
    let c = resolve_coefficients("(unknown)", &BTreeMap::new());
    let e = estimate_session_energy(&s, &c);
    assert!((e.total_wh - 1.0).abs() < 0.01); // unknown tier: 1.0 Wh/1K output
}
```

- [ ] **Step 2: Run tests, verify failure.** `cargo test --test energy_test` — expected: compile error (module not found).

- [ ] **Step 3: Implement.** Create `src/analysis/energy.rs` with the coefficient table, tier mapping, and estimation functions per the spec. Add `pub mod energy;` to `src/analysis/mod.rs`. Add `EnergyConfig` and `EnergyOverride` to `src/ethics/manifest.rs` under `Thresholds` with `#[serde(default)]`.

The coefficient table: a function `default_tier_table() -> Vec<(&str, &str, EnergyCoefficients)>` where each entry is `(model_prefix, tier_name, coefficients)`. Order matters (first prefix match wins). Include at minimum:
- `"claude-opus"` → frontier
- `"claude-fable"` / `"claude-sonnet"` → large
- `"claude-haiku"` → medium
- `"gpt-4.1-nano"` / `"gemini-flash"` → small
- `"gpt-4o-mini"` / `"gpt-4.1-mini"` → medium
- `"gpt-4"` / `"gpt-4o"` / `"gpt-4.1"` → large (after mini matches)
- `"o1"` / `"o3"` → reasoning
- Fallthrough → unknown

`resolve_coefficients(model, overrides)`: find the tier, build `EnergyCoefficients`, apply any `EnergyOverride` fields that are `Some`.

`estimate_session_energy(session, coefficients)`: multiply each token field by the corresponding coefficient / 1000.

`estimate_total_energy(sessions, config)`: iterate sessions, resolve coefficients per model (using `session.model.as_deref().unwrap_or("(unknown)")`), compute per-session energy, aggregate per-model and total, compute weighted uncertainty, compute CO2 if `grid_carbon_intensity` is set, build methodology string.

- [ ] **Step 4: Run tests, all pass.** `cargo test` — full suite green.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Add energy estimation module with per-model coefficients"`

---

### Task 2: `conscience report energy` CLI command

**Files:**
- Modify: `src/main.rs` (add `Energy` variant to `ReportSource`, add `run_energy_report`)
- Create: `src/analysis/energy_report.rs` (terminal rendering)
- Modify: `src/analysis/mod.rs` (add `pub mod energy_report;`)
- Modify: `README.md`, `CLAUDE.md` (document the command)

**Interfaces:**
- Consumes: `estimate_total_energy(&sessions, &config) -> EnergyEstimate` from Task 1.
- Produces: `print_energy_report(&EnergyEstimate)`, CLI subcommand `conscience report energy [--project <path>] [--days 30] [--json]`.

- [ ] **Step 1: Add `Energy` variant to `ReportSource`:**

```rust
/// Report on estimated energy consumption of AI usage
Energy {
    /// Filter to a specific project directory
    #[arg(long)]
    project: Option<PathBuf>,
    /// Number of days to look back
    #[arg(long, default_value = "30")]
    days: u32,
    /// Output as JSON
    #[arg(long)]
    json: bool,
},
```

Add match arm: `ReportSource::Energy { project, days, json } => run_energy_report(project.as_deref(), days, json),`

- [ ] **Step 2: Implement `run_energy_report`** in `main.rs`:

```rust
fn run_energy_report(
    project: Option<&std::path::Path>,
    days: u32,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = ingest::ai::ingest_claude_code(project)?;
    if summary.session_count == 0 {
        eprintln!("No Claude Code sessions found.");
        std::process::exit(1);
    }

    let cwd = std::env::current_dir()?;
    let manifest_dir = project.unwrap_or(&cwd);
    let manifest = ethics::manifest::Manifest::load(manifest_dir);
    let config = manifest
        .as_ref()
        .map(|m| m.thresholds.energy.clone())
        .unwrap_or_default();

    // Filter sessions to the time window
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let sessions: Vec<_> = summary.sessions.iter()
        .filter(|s| s.started_at.map_or(true, |t| t >= cutoff))
        .collect();

    let estimate = analysis::energy::estimate_total_energy(
        &sessions.iter().map(|s| (*s).clone()).collect::<Vec<_>>(),
        &config,
    );

    if json_output {
        println!("{}", serde_json::to_string_pretty(&estimate)?);
    } else {
        analysis::energy_report::print_energy_report(&estimate);
    }

    Ok(())
}
```

- [ ] **Step 3: Create `src/analysis/energy_report.rs`** — terminal rendering per the spec's example output (honesty header, per-model table, totals with uncertainty, laptop-hour equivalent, CO2 if set, sources).

- [ ] **Step 4: Build, run manually.** `cargo build && cargo run -- report energy --days 30` — verify output looks right against real data. Also `--json | head -20`.

- [ ] **Step 5: Update docs.** README: add "Energy Reports" section after "AI Tool Usage Reports" with example command. CLAUDE.md: add `conscience report energy [--project <path>] [--days 30] [--json]` to the commands block.

- [ ] **Step 6: Run full suite, commit.** `cargo test && git add -A && git commit -m "Add conscience report energy command"`

---

### Task 3: Enrich EnvironmentalCost signal + proportionality reflection

**Files:**
- Modify: `src/ethics/signals.rs` (`detect_token_consumption`)
- Modify: `src/ethics/reflection.rs` (`build_proportionality_reflection`)
- Modify: `tests/signals_test.rs` (extend)

**Interfaces:**
- Consumes: `resolve_coefficients()`, `estimate_total_energy()` from Task 1.
- Produces: enriched signal detail/evidence strings; enriched reflection data_context.

- [ ] **Step 1: Write the failing test** — append to `tests/signals_test.rs`:

```rust
#[test]
fn test_token_consumption_signal_includes_energy_estimate() {
    let session = make_ai_session("s1", 2_000_000, 10, 15, 5, 3, vec![], vec![], 1.0);
    let summary = make_ai_summary(vec![session]);
    let signals = signals::detect_ai_signals(&summary, None);

    let token_signal = signals.iter().find(|s| s.title.contains("token consumption"));
    assert!(token_signal.is_some(), "should have token consumption signal");
    let sig = token_signal.unwrap();
    assert!(sig.detail.contains("Wh"), "detail should mention Wh, got: {}", sig.detail);
    assert!(sig.evidence.contains("laptop"), "evidence should have laptop comparison, got: {}", sig.evidence);
}
```

- [ ] **Step 2: Run, verify failure.** The signal exists but detail/evidence don't mention Wh yet.

- [ ] **Step 3: Implement.** In `detect_token_consumption`, after the existing `output > 1_000_000` block constructs the signal, compute the energy estimate inline:

```rust
// Inside the output > 1_000_000 block, enrich the signal:
let energy_config = conscience::ethics::manifest::EnergyConfig::default();
let est = conscience::analysis::energy::estimate_total_energy(
    &summary.sessions, &energy_config,
);
// Append to detail:
detail = format!("{} Estimated energy: ~{:.0} Wh (±{:.0}%).",
    detail, est.total_wh, /* blended uncertainty */);
// Append to evidence:
evidence = format!("{}, roughly {:.1} hours of laptop use",
    evidence, est.total_wh / 60.0);
```

Similarly in `build_proportionality_reflection`, add energy context to `context_parts` when AI data is available.

- [ ] **Step 4: Run tests, all pass.** `cargo test`

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Enrich EnvironmentalCost signal and reflection with energy estimates"`

---

### Task 4: Version bump, publish, close roadmap issues

**Files:**
- Modify: `Cargo.toml` (version → 0.3.0)

- [ ] **Step 1: Bump version.** Edit `Cargo.toml` version to `"0.3.0"`.

- [ ] **Step 2: Full suite, build, manual smoke test.** `cargo test && cargo build && cargo run -- report energy --days 7`

- [ ] **Step 3: Commit, tag, push.** `git add -A && git commit -m "Bump version to 0.3.0" && git tag v0.3.0 && git push origin main v0.3.0`

- [ ] **Step 4: Publish.** `cargo publish`

- [ ] **Step 5: Close roadmap issues.** `gh issue close 348 349 350 351 -R subversivesoftwareorg/roadmap` with comments linking the release.

---

## Self-Review

**Spec coverage:**
- Research coefficients (§Research basis, §Coefficient table) → Task 1 coefficient table ✓
- Energy module (§Architecture → Energy module) → Task 1 ✓
- Configuration (§Architecture → Configuration) → Task 1 `EnergyConfig`/`EnergyOverride` ✓
- `report energy` command (§Architecture → CLI) → Task 2 ✓
- Signal enrichment (§Architecture → Signal enrichment) → Task 3 ✓
- Reflection enrichment (§Architecture → Reflection enrichment) → Task 3 ✓
- Honest-measurement caveats → Tasks 1-2 (methodology string, header caveats) ✓
- Privacy → no new data extracted ✓
- Testing → Task 1 (10 tests covering all spec scenarios) ✓

**Placeholder scan:** No TBD/TODO found. All test code is concrete with hand-verified expected values.

**Type consistency:** `EnergyConfig` used consistently across Tasks 1-3. `estimate_total_energy(&[AiSession], &EnergyConfig) -> EnergyEstimate` signature stable. `resolve_coefficients(model: &str, overrides: &BTreeMap<String, EnergyOverride>) -> EnergyCoefficients` stable.
