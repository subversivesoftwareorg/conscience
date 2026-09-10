# Energy Accounting — Design

**Date:** 2026-09-10
**Status:** Approved design, pending implementation plan
**Roadmap issues:** #348 (research), #349 (energy module), #350 (report energy command), #351 (EnvironmentalCost signal enrichment)

## Purpose

Make the EnvironmentalCost principle measurable by mapping token usage to
estimated energy consumption (watt-hours), summarized per model, per session,
and as a monthly/period total. This serves MH 101: "AI systems require
enormous amounts of energy and water."

No provider publishes official per-model energy data. Every coefficient in
this system is a third-party estimate. The design's honesty contract: label
all figures as estimates, carry uncertainty ranges, cite sources, and never
present a single number as fact.

## Research basis

Coefficients are drawn from:

1. **Jegham et al. (2025)** "How Hungry is AI?" arXiv:2505.09598 — per-model
   per-query benchmarks (Claude 3.7 Sonnet: 0.84 Wh short query; GPT-4o:
   0.42 Wh; o3: 7.03 Wh).
2. **mdodkins (2026)** Claude Code energy gist — per-token-type breakdown
   (input: 390 Wh/M, output: 1,950 Wh/M, cache create: 490 Wh/M, cache
   read: 39 Wh/M).
3. **Patterson et al. (2025)** Joule — median 0.31 Wh/query for frontier
   models.
4. **IEA (2025)** — Google Gemini at 0.24 Wh, ChatGPT at ~0.34 Wh per
   median query.
5. **Luccioni et al. (2024)** FAccT — foundational inference energy
   methodology.

Key ratios confirmed across sources:
- Output tokens: ~5x input token cost (API-level; 10–30x at hardware level)
- Cache reads: ~0.1x input token cost
- Cache creation: ~1.25x input token cost

## Coefficient table

Each model is mapped to a tier. The table lives in code (not config) with
user overrides in `conscience.yaml`.

| Tier | Example models | Wh/1K output | Wh/1K input | Wh/1K cache create | Wh/1K cache read | Uncertainty | Source note |
|---|---|---|---|---|---|---|---|
| Frontier | Opus 5/4, GPT-4.5, o3 | 3.0 | 0.60 | 0.75 | 0.06 | ±100% | Jegham (o3: 7.03 Wh/short) |
| Large | Fable 5, Sonnet 5/4, GPT-4o, GPT-4.1 | 1.5 | 0.30 | 0.375 | 0.03 | ±50% | mdodkins, Jegham (Sonnet 3.7: 0.84 Wh/short) |
| Medium | Haiku 4.5, GPT-4o-mini, GPT-4.1-mini | 0.5 | 0.10 | 0.125 | 0.01 | ±50% | Jegham (4o-mini: 0.42 Wh/short) |
| Small | GPT-4.1-nano, Gemini Flash | 0.15 | 0.03 | 0.0375 | 0.003 | ±50% | Jegham (nano: 0.10 Wh/short) |
| Reasoning | o1, o3, extended thinking modes | 5.0 | 0.60 | 0.75 | 0.06 | ±150% | Jegham (o3: 7.03, o1: 4.45 Wh/short) |
| Unknown | Copilot, Cursor, Windsurf, unrecognized | 1.0 | 0.20 | 0.25 | 0.02 | ±200% | Blended estimate |

Model-to-tier mapping is a list of `(pattern, tier)` pairs checked in order.
Pattern matching: prefix match on the model string (e.g., `"claude-opus"` matches
`"claude-opus-5"`, `"claude-opus-4-6"`). First match wins. Unmatched models
fall to the Unknown tier with a stderr note.

Cache-create coefficient = 1.25 × input coefficient per tier.
Cache-read coefficient = 0.1 × input coefficient per tier.

## Architecture

### Energy module (`src/analysis/energy.rs`)

Pure functions over `&[AiSession]` + config.

```rust
pub struct EnergyCoefficients {
    pub wh_per_1k_input: f64,
    pub wh_per_1k_output: f64,
    pub wh_per_1k_cache_create: f64,
    pub wh_per_1k_cache_read: f64,
    pub uncertainty_pct: f64,
    pub tier: String,
    pub source_note: String,
}

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

pub struct ModelEnergy {
    pub model: String,
    pub tier: String,
    pub sessions: u64,
    pub total_wh: f64,
    pub uncertainty_pct: f64,
    pub tokens: TokenUsage,
}

pub struct EnergyEstimate {
    pub period_days: u32,
    pub per_model: Vec<ModelEnergy>,
    pub per_session: Vec<SessionEnergy>,
    pub total_wh: f64,
    pub total_kwh: f64,
    pub blended_wh_per_1k_output: f64,
    pub uncertainty_range: (f64, f64),  // (low, high) in Wh
    pub co2_kg: Option<f64>,           // if grid intensity set
    pub grid_carbon_intensity: Option<f64>,
    pub methodology: String,
}

pub fn resolve_coefficients(model: &str, overrides: &BTreeMap<String, EnergyOverride>) -> EnergyCoefficients
pub fn estimate_session_energy(session: &AiSession, coefficients: &EnergyCoefficients) -> SessionEnergy
pub fn estimate_total_energy(sessions: &[AiSession], config: &EnergyConfig) -> EnergyEstimate
```

All structs derive `Debug, Clone, Serialize, Deserialize`.

`estimate_session_energy` computes per-token-type Wh:
```
input_wh  = tokens.input  * coefficients.wh_per_1k_input  / 1000.0
output_wh = tokens.output * coefficients.wh_per_1k_output / 1000.0
cache_create_wh = tokens.cache_creation * coefficients.wh_per_1k_cache_create / 1000.0
cache_read_wh   = tokens.cache_read * coefficients.wh_per_1k_cache_read / 1000.0
total_wh = input_wh + output_wh + cache_create_wh + cache_read_wh
```

`uncertainty_range` = `(total_wh * (1 - blended_uncertainty), total_wh * (1 + blended_uncertainty))` where blended uncertainty is the
weighted average of per-session uncertainties by Wh contribution.

CO2 estimate: `total_wh / 1000.0 * grid_carbon_intensity` when the grid
intensity is set (either config or default).

`methodology` string cites the sources and version of the coefficient table.

### Configuration (`src/ethics/manifest.rs`)

```rust
pub struct EnergyConfig {
    pub overrides: BTreeMap<String, EnergyOverride>,
    pub grid_carbon_intensity: Option<f64>,
}

pub struct EnergyOverride {
    pub wh_per_1k_input: Option<f64>,
    pub wh_per_1k_output: Option<f64>,
}
```

Nested under `Thresholds.energy` with `#[serde(default)]`. Override
fields are `Option` — only the fields the user sets replace the
tier defaults; unset fields keep the tier value.

### CLI: `conscience report energy`

`conscience report energy [--project <path>] [--days 30] [--json]`

Terminal output:

```
  Conscience — Energy Estimate (30 day window)
  Based on third-party research. No provider publishes official energy data.
  All figures are estimates with stated uncertainty ranges.

  Per-Model Breakdown
  ╭─────────────────────┬──────────┬─────────┬───────────────┬──────────────╮
  │ Model               │ Sessions │ Tokens  │ Est. Wh       │ Uncertainty  │
  ├─────────────────────┼──────────┼─────────┼───────────────┼──────────────┤
  │ claude-fable-5      │ 42       │ 2.1M    │ 245 ± 50%     │ Large tier   │
  │ claude-opus-5       │ 3        │ 0.4M    │ 89 ± 100%     │ Frontier     │
  ╰─────────────────────┴──────────┴─────────┴───────────────┴──────────────╯

  Totals
    Estimated energy: 334 Wh (range: 167–668 Wh)
    Equivalent to: ~5.6 hours of laptop use (60W)
    Blended efficiency: 1.3 Wh per 1K output tokens
    Estimated CO2: 0.14 kg (at 0.42 kgCO2/kWh US avg)

  Sources: Jegham et al. 2025, mdodkins 2026, Patterson et al. 2025
```

The "equivalent to" line converts Wh to a relatable comparison (hours of
laptop at 60W). CO2 line only appears when `grid_carbon_intensity` is set
(defaults to 0.42 kgCO2/kWh US average; users override in
`conscience.yaml`).

### Signal enrichment (`src/ethics/signals.rs`)

`detect_token_consumption` gains an energy estimate when token data is
available: the existing "Significant token consumption" signal's detail
gains "~X Wh estimated energy" and the evidence gains "roughly equivalent
to Y hours of laptop use." No new signal — the existing one gets richer
data. The energy module is called inline (it's a pure function over the
same `AiUsageSummary` the signal already has).

### Reflection enrichment (`src/ethics/reflection.rs`)

`build_proportionality_reflection` gains energy context when available:
the `data_context` field adds "estimated ~X Wh energy consumed" alongside
the existing token count.

## Testing

- TDD throughout.
- `tests/energy_test.rs`: coefficient lookup (known model, prefix match,
  fallback to Unknown, override application), session energy arithmetic
  (hand-verified: 1000 output tokens of a Large model = 1.5 Wh), total
  aggregation across mixed models, uncertainty propagation
  (single-tier = tier uncertainty, mixed tiers = weighted blend), CO2
  with and without grid intensity.
- Signal enrichment: extend existing signal tests to check the energy
  data appears in the signal detail/evidence.
- Edge cases: session with no model field (Unknown tier), session with
  zero tokens (zero energy, not NaN), all-cache session (mostly cache-read
  energy).

## Privacy

Energy data is derived from token counts and model names already present in
the analysis pipeline. No new data is extracted from session logs. The
coefficient table and methodology are public (cited research).

## Honest-measurement caveats (shown to users)

- All figures are third-party estimates based on published research, not
  provider-disclosed data.
- Uncertainty ranges reflect measurement confidence, not precision —
  actual energy could be anywhere in the stated range.
- Hardware, data center location, batch size, and serving optimization
  can vary actual energy by 2–5x within the same model class.
- CO2 estimates compound two uncertainties (energy × grid intensity) and
  are labeled as such.
- The "equivalent to X hours of laptop" comparison uses 60W as a round
  number for a developer laptop under load; actual laptop power draw
  varies.

## Out of scope (future)

- Water consumption estimates (MH 101 also mentions water).
- Per-file or per-tool energy attribution (which files cost the most
  energy to produce).
- Historical trend tracking via conscience-dashboard.
- Renewable energy offset tracking.
