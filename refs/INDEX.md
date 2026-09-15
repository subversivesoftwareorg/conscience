# Reference Documents

Papers, encyclicals, declarations, and standards that ground conscience's ethical framework and measurement methodology. Each entry notes where it is cited in the codebase and whether a PDF is included.

**Status key:** `[PDF]` included in this repo | `[manual download]` freely available but requires browser | `[paid]` requires purchase | `[web]` web-based content, no PDF

---

## Ethical Framework (current)

### Magnifica Humanitas (Leo XIV, May 2026)
**File:** `Encyclical Letter of His Holiness Leo XIV Magnifica Humanitas (15 May 2026).pdf` `[PDF]`

Encyclical on safeguarding the human person in the time of artificial intelligence. The foundational document for conscience's seven ethical principles: human dignity (MH 52), common good (MH 60), subsidiarity (MH 70), solidarity (MH 73), social justice (MH 77), environmental cost (MH 101), de-skilling (MH 150).

**Cited in:** `CLAUDE.md`, `src/ethics/models.rs`, `src/ethics/reflection.rs`, wiki

### Leiden Declaration on AI and Mathematics (June 2026)
**File:** `Leiden_Declaration_on_Artificial_Intelligence_and_Mathematics.pdf` `[PDF]`

Practical recommendations from the mathematical community: transparently disclose tool use (O1), retain responsibility for correctness (O4), affirm human authorship (O5), proper attribution (O6).

**Cited in:** `CLAUDE.md`, `src/ethics/models.rs`

---

## Energy & Environmental Cost

### Jegham et al. (2025) — "How Hungry is AI?"
**File:** `energy/Jegham_2025_How_Hungry_is_AI.pdf` `[PDF]`
**Source:** arXiv:2505.09598

Most comprehensive per-model energy benchmarks. Per-query Wh for Claude 3.7 Sonnet (0.84), GPT-4o (0.42), o3 (7.03), and others. Primary source for conscience's energy tier coefficients.

**Cited in:** `src/analysis/energy.rs`, energy accounting spec

### Luccioni et al. (2024) — "Power Hungry Processing"
**File:** `energy/Luccioni_2024_Power_Hungry_Processing.pdf` `[PDF]`
**Source:** FAccT '24 (arXiv:2311.02468)

Foundational inference energy methodology. Validates output:input energy ratio (10-30x per token at hardware level).

**Cited in:** energy accounting spec

### Patterson et al. (2025) — "Energy Use of AI Inference"
**Source:** Joule `[paid]`

Median 0.31 Wh per query for frontier models. Cross-validates Jegham benchmarks from the infrastructure side.

**Cited in:** `src/analysis/energy.rs`, energy accounting spec

### de Vries (2023) — "The Growing Energy Footprint of Artificial Intelligence"
**File:** `energy/de_Vries_2023_Growing_Energy_Footprint.pdf` `[PDF]`
**Source:** Joule (arXiv:2311.16863)

Macro-level projections: AI could consume 85-134 TWh annually by 2027 (0.5% of global electricity).

**Cited in:** energy accounting spec

### Li et al. (2023) — "Making AI Less Thirsty"
**File:** `energy/Li_2023_Making_AI_Less_Thirsty.pdf` `[PDF]`
**Source:** arXiv:2304.03271

Water consumption of AI inference. GPT-3 training consumed ~700,000 liters. Provides liters-per-kWh coefficients (US avg 1.8 L/kWh). Source for conscience's water estimation.

**Cited in:** `src/analysis/energy.rs`

### TokenPowerBench (2024)
**File:** `energy/TokenPowerBench_2024.pdf` `[PDF]`
**Source:** arXiv:2512.03024

Hardware-level per-token energy measurements across GPU architectures. Validates 10-30x output:input ratio.

**Cited in:** energy accounting spec

### ISO/IEC TR 20226:2025 — Environmental Sustainability of AI Systems
**Source:** https://www.iso.org/standard/86177.html `[paid]` (CHF 204, 61 pages)

International standard for AI environmental sustainability metrics. Covers carbon, water, waste across the full lifecycle with Scope 1/2 distinctions and geographic variation. Would standardize conscience's environmental methodology.

**Gap it fills:** Lifecycle framing beyond inference-time, geographic grid variation, standardized metric definitions.

### mdodkins (2026) — Claude Code Energy Estimate
**Source:** GitHub gist `[web]`

Per-token-type energy breakdown: input 390 Wh/M, output 1,950 Wh/M, cache creation 490, cache read 39. Primary source for Large-tier coefficients.

**Cited in:** `src/analysis/energy.rs`

### IEA (2025) — "Key Questions on Energy and AI"
**Source:** iea.org `[web]`

Google Gemini at 0.24 Wh, ChatGPT at ~0.34 Wh per median query.

**Cited in:** energy accounting spec

---

## Labor & Developer Impact

### Stanford HAI 2026 AI Index Report (April 2026)
**Source:** https://aiindex.stanford.edu/report/ `[manual download]`

Using ADP payroll data, documents ~20% decline in employment for software developers aged 22-25 since 2024. Junior employment drops 9-10% within six quarters of AI tool adoption. Transforms MH's de-skilling warning from philosophy to measured fact.

**Gap it fills:** Empirical ground truth for Developer Growth reflection questions; contextualize team junior-to-senior ratios against industry trends.

### ILO (2025) — "Work Transformed: Promise and Peril of AI"
**File:** `labor/ILO_2025_Work_Transformed.pdf` `[PDF]`
**Source:** ILO Research Brief

Global labor framework distinguishing automation (technology performs tasks) from augmentation (technology improves human performance). Documents gender disparities in AI exposure.

**Gap it fills:** Automation vs augmentation distinction conscience could operationalize; systemic displacement via skill-network adjacency.

### ILO (2026) — "Workers' Exposure to AI"
**File:** `labor/ILO_2026_Workers_Exposure_to_AI.pdf` `[PDF]`
**Source:** ILO Research Brief

Occupational exposure indices for generative AI. Measures which roles are most exposed to automation vs augmentation.

**Gap it fills:** Quantified exposure metrics for different developer roles.

---

## Governance & Engineering Practice

### NIST AI 600-1: Generative AI Profile (July 2024)
**File:** `governance/NIST_AI_600-1_GenAI_Profile_2024.pdf` `[PDF]`
**Source:** https://doi.org/10.6028/NIST.AI.600-1

Operational risk taxonomy with 200+ concrete actions across 12 GenAI risk categories using GOVERN/MAP/MEASURE/MANAGE structure. Value Chain and Component Integration category maps to code provenance concerns.

**Gap it fills:** Structured risk assessment methodology; conscience's signals could be organized into NIST risk categories.

### Responsible AI Pattern Catalogue (ACM Computing Surveys, April 2024)
**File:** `governance/RAI_Pattern_Catalogue_2024.pdf` `[PDF]`
**Source:** arXiv:2209.04963

63 concrete engineering patterns in three tiers: multi-level governance (24), trustworthy process (17), RAI-by-design product (22). Bridges principles to software engineering practice.

**Gap it fills:** Maps measurements to actionable engineering patterns (e.g., "is the team following the human-in-the-loop pattern?").

### IEEE 7000 — Model Process for Addressing Ethical Concerns (2021)
**Source:** https://standards.ieee.org/ieee/7000/6781/ `[paid]`

Bottom-up value elicitation process for engineering teams. Doesn't prescribe values but provides methodology for teams to identify and operationalize their own.

**Gap it fills:** A "how" complement to MH's "what" — teams could use IEEE 7000's value elicitation in conscience's reflection features.

### EU AI Act + OECD AI and Work Program (2024-2026)
**Source:** https://eur-lex.europa.eu/eli/reg/2024/1689 `[web]`

Enforceable obligations: high-risk rules apply August 2026, copyright compliance policies required since August 2025. OECD AILit Framework addresses AI literacy requirements.

**Gap it fills:** Compliance dimension — is AI usage meeting legal obligations, not just ethical aspirations?

---

## Data Sovereignty & Decolonial Ethics

### CARE Principles for Indigenous Data Governance (2019)
**Source:** https://datascience.codata.org/articles/dsj-2020-043 `[manual download]`
**Authors:** Global Indigenous Data Alliance / Research Data Alliance

Collective Benefit, Authority to Control, Responsibility, Ethics. Challenges the assumption that data openness is inherently good. Full disclosure of AI usage might conflict with data sovereignty if training data involves Indigenous knowledge.

**Gap it fills:** Collective benefit governance; extends MH's common good with community consent mechanisms.

### "AI Ethics Through a Decolonial Lens" (February 2026)
**Source:** https://pmc.ncbi.nlm.nih.gov/articles/PMC13385455/ `[manual download]`
**Published in:** AI & Society (Springer)

Three reorientations: (1) AI as means to redress power asymmetries, not just "do no harm"; (2) AI grounded in local data sovereignty; (3) AI as relationally entangled with humans and Earth.

**Gap it fills:** Non-Western epistemological framing; evaluates whether AI tools reproduce colonial knowledge hierarchies.

### "African Data Ethics: A Discursive Framework for Black Decolonial AI" (FAccT 2025)
**Source:** ACM FAccT '25 proceedings `[manual download]`
**Authors:** Barrett, Okolo, Biira, Sherif, Zhang, Battle

Ubuntu-informed relational ethics. Communal rather than individual conceptions of dignity. Attention to how AI reproduces structural exclusion.

**Gap it fills:** Global South perspective; broadens equity analysis beyond team-internal fairness.

---

## Commons & Licensing

### Open Future — Public AI & Commons Framework (2025-2026)
**Source:** https://openfuture.eu/publication/white-paper-on-public-ai/ `[manual download]`
**Author:** Open Future (European think tank), commissioned by Bertelsmann Stiftung

"Gradient of publicness" for AI infrastructure. Identifies "data winter" — declining willingness to share data as proprietary actors secure exclusive datasets.

**Gap it fills:** Reciprocity dimension — is the team contributing back to the open-source commons their AI tools were trained on?

### Creative Commons "CC Signals" Framework (September 2026)
**Source:** https://creativecommons.org/2026/09/03/guidance-on-using-cc-licenses-in-an-ai-ecosystem/ `[web]`

Machine-readable preference signals for AI use of content and data. Operationalizes the attribution that Leiden calls for.

**Gap it fills:** Practical mechanisms for code provenance and attribution tracking in AI contexts.
