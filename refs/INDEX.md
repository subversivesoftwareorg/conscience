# Reference Documents

Papers, encyclicals, and declarations that ground conscience's ethical framework and measurement methodology. Each entry notes where it is cited in the codebase.

## Ethical Framework

### Magnifica Humanitas (Leo XIV, May 2026)
**File:** `Encyclical Letter of His Holiness Leo XIV Magnifica Humanitas (15 May 2026).pdf`

Encyclical on safeguarding the human person in the time of artificial intelligence. The foundational document for conscience's seven ethical principles:
- **Human Dignity** (MH 52) — human value does not depend on output
- **Common Good** (MH 60) — a greater good that belongs to everyone
- **Subsidiarity** (MH 70) — decisions at the closest level to the persons involved
- **Solidarity** (MH 73) — no one is saved alone
- **Social Justice** (MH 77) — institutions must serve all persons
- **Environmental Cost** (MH 101) — AI requires enormous energy and water
- **De-skilling** (MH 150) — technology can de-skill workers and subject them to surveillance

**Cited in:** `CLAUDE.md`, `src/ethics/models.rs` (principle sources), `src/ethics/reflection.rs` (reflection questions), wiki (The Seven Principles)

### Leiden Declaration on AI and Mathematics (June 2026)
**File:** `Leiden_Declaration_on_Artificial_Intelligence_and_Mathematics.pdf`

Practical recommendations from the mathematical community:
- O1: Transparently disclose use of automated tools
- O4: Retain human responsibility for correctness
- O5: Affirm the humanity of authorship
- O6: Put effort into proper attribution

**Cited in:** `CLAUDE.md`, `src/ethics/models.rs` (Transparency, Code Provenance, Security principles)

## Energy & Environmental Cost

### Jegham et al. (2025) — "How Hungry is AI?"
**File:** `energy/Jegham_2025_How_Hungry_is_AI.pdf`
**Source:** arXiv:2505.09598

Most comprehensive per-model energy benchmarks. Provides Wh-per-query data for Claude 3.7 Sonnet (0.84 Wh), GPT-4o (0.42 Wh), o3 (7.03 Wh), and many others across short/medium/long query sizes. Primary source for conscience's energy tier coefficients.

**Cited in:** `src/analysis/energy.rs` (tier table source notes), `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

### Luccioni et al. (2024) — "Power Hungry Processing"
**File:** `energy/Luccioni_2024_Power_Hungry_Processing.pdf`
**Source:** FAccT '24 (arXiv:2311.02468)

Foundational inference energy methodology. Establishes the framework for measuring AI inference energy consumption across model sizes and tasks. Validates the output:input energy ratio (output tokens are 10-30x more expensive per token).

**Cited in:** `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

### Patterson et al. (2025) — "Energy Use of AI Inference"
**Source:** Joule (no open-access PDF available)

Median 0.31 Wh per query for frontier models. Cross-validates the Jegham benchmarks from the infrastructure side. Not available as a downloadable PDF — published in Joule behind a paywall.

**Cited in:** `src/analysis/energy.rs` (methodology string), `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

### de Vries (2023) — "The Growing Energy Footprint of Artificial Intelligence"
**File:** `energy/de_Vries_2023_Growing_Energy_Footprint.pdf`
**Source:** Joule (arXiv:2311.16863)

Macro-level projections of AI energy consumption growth. Estimates that AI could consume 85-134 TWh annually by 2027 (0.5% of global electricity). Provides context for why per-session energy measurement matters.

**Cited in:** `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

### Li et al. (2023) — "Making AI Less Thirsty"
**File:** `energy/Li_2023_Making_AI_Less_Thirsty.pdf`
**Source:** arXiv:2304.03271

Water consumption of AI training and inference. Finds GPT-3 training consumed ~700,000 liters. Provides the liters-per-kWh coefficients used in conscience's water estimation (US avg 1.8 L/kWh, varying by cooling method and region).

**Cited in:** `src/analysis/energy.rs` (methodology string), `src/ethics/manifest.rs` (water_liters_per_kwh default)

### TokenPowerBench (2024)
**File:** `energy/TokenPowerBench_2024.pdf`
**Source:** arXiv:2512.03024

Hardware-level per-token energy measurements across GPU architectures. Validates that output token generation is 10-30x more expensive than input processing at the hardware level, supporting the 5:1 API-level ratio used in conscience's coefficients.

**Cited in:** `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

## Third-Party Estimates

### mdodkins (2026) — Claude Code Energy Estimate
**Source:** GitHub gist (no PDF)

Per-token-type energy breakdown for Claude Code sessions: input 390 Wh/M tokens, output 1,950 Wh/M tokens, cache creation 490 Wh/M, cache read 39 Wh/M. Primary source for the Large-tier coefficients and the cache-read discount.

**Cited in:** `src/analysis/energy.rs` (tier table), `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`

### IEA (2025) — "Key Questions on Energy and AI"
**Source:** iea.org (no PDF downloaded — web report)

Google Gemini at 0.24 Wh per median query, ChatGPT at ~0.34 Wh. Cross-validates the academic benchmarks from an institutional source.

**Cited in:** `docs/superpowers/specs/2026-09-10-energy-accounting-design.md`
