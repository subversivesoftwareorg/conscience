# Energy coefficient verification against refs/energy

Date: 2026-09-21. Scope: every constant and methodological claim in
`src/analysis/energy.rs`, `src/analysis/comparisons.rs`, the energy design
spec, `refs/INDEX.md`, and the wiki's Measuring Energy Cost page, checked
against the PDFs in `refs/energy/` and, where a constant is attributed to a
web source, that source. Leiden: "don't believe the hype; verify claims
against evidence." No code changes.

## 0. The reference library itself has problems

Status: the two file mix-ups below were fixed the same day (the real Luccioni PDF now sits under its own name, the de Vries commentary was added, INDEX IDs corrected). The rest is tracked in the roadmap.

| File | What it actually is | Verdict |
|---|---|---|
| `Luccioni_2024_Power_Hungry_Processing.pdf` | "Relativistic electron precipitation events driven by solar wind impact on the Earth's magnetosphere" (Roosnovo et al., arXiv:2311.02468, space physics) | Wrong paper. INDEX gives arXiv:2311.02468 for Luccioni; that ID is this physics paper. |
| `de_Vries_2023_Growing_Energy_Footprint.pdf` | Luccioni, Jernite, Strubell, "Power Hungry Processing: Watts Driving the Cost of AI Deployment?", FAccT '24, arXiv:2311.16863 | Mislabelled. This is the real Luccioni paper. |
| de Vries (2023), Joule 7(10):2191–2194, doi:10.1016/j.joule.2023.09.004 | Not present anywhere in `refs/`. It has no arXiv ID; INDEX's "arXiv:2311.16863" is Luccioni's. | Missing. |
| `Jegham_2025_How_Hungry_is_AI.pdf` | arXiv:2505.09598 **v6**, 24 Nov 2025, 30 models | Correct paper, but the numbers quoted in code and INDEX are from an earlier version (see §1). |
| `Li_2023_Making_AI_Less_Thirsty.pdf` | arXiv:2304.03271 v5, 26 Mar 2025 | Correct. |
| `TokenPowerBench_2024.pdf` | Niu et al., arXiv:2512.03024 (Dec 2025; AAAI 2026) | Correct; the "2024" in the filename is wrong. |
| "Patterson et al. (2025), Joule, median 0.31 Wh" | The paper is Elsworth, Patterson et al., "Measuring the environmental impact of delivering AI at Google Scale", arXiv:2508.15734 (Aug 2025). Not Joule. Its median is **0.24 Wh** per Gemini Apps text prompt, 0.26 mL water. 0.31 Wh appears nowhere. | Mis-cited; the number is wrong. |
| "IEA (2025): Gemini 0.24 Wh, ChatGPT ~0.34 Wh" | 0.24 Wh is Google's own figure (paper above). 0.34 Wh is Sam Altman's June 2025 blog post, which Jegham §5.3 quotes. Neither is an IEA measurement. | Mis-attributed. |
| "mdodkins (2026)" gist | Real: gist.github.com/mdodkins/9b49624855cc41570c9d1012e0d5d157, "Claude Code Energy Use Estimate (9 March 2026)". Table: input 390, output 1,950, cache create 490, cache read 39 Wh/M tokens. The gist gives **no derivation**; its base figures come from Simon Couch's blog, which blends an Epoch AI estimate for GPT-4o; the cache ratios are exactly Anthropic's price ratios (1.25×, 0.1×). | Secondary source; not a measurement. |

Everything conscience cites as "Luccioni validates the 10–30× output:input
ratio at hardware level" and "TokenPowerBench validates 10–30×" is
unsupported by either paper as written (§3).

## 1. Constant-by-constant verification

Jegham v6 Table 4 (p. 8) gives per-query Wh at three prompt shapes:
(100 in, 300 out), (1,000 in, 1,000 out), (10,000 in, 1,500 out). A
least-squares fit of E = a·in/1K + b·out/1K over the three points separates
input from output cost. Figures are facility-level (PUE 1.12–1.14 included).

| Model (Jegham v6) | in Wh/1K | out Wh/1K | out:in |
|---|---|---|---|
| Claude 3.7 Sonnet | 0.135 | 2.88 | 21 |
| Claude 3.5 Sonnet | 0.276 | 3.34 | 12 |
| Claude 3.5 Haiku | 0.173 | 4.19 | 24 (but see below) |
| GPT-4o (Mar '25) | 0.119 | 1.12 | 9 |
| GPT-4.1 | ~0 | 3.12 | – |
| GPT-4.1 mini | ~0 | 1.56 | – |
| GPT-4.1 nano | ~0 | 0.59 | – |
| GPT-4o mini | 0.029 | 1.87 | – |
| o3 | 0.543 | 4.53 | 8 |
| o1 | ~0 | 4.41 | – |
| GPT-5 minimal reasoning (Fig. 6, p. 12) | 0.414 | 1.93 | 5 |
| GPT-5 low | 0.713 | 4.74 | 7 |
| GPT-5 medium | 0.287 | 11.2 | 39 |
| GPT-5 high | 0.673 | 18.0 | 27 |

Where the fitted input coefficient is ~0 or negative, the three points are
explained by output alone plus a fixed per-query overhead (latency to first
token); Jegham's model is time-based, so input tokens cost only through
latency.

### Tier table in `energy.rs:73-91`

| Constant | In code | What the cited source says | Verdict |
|---|---|---|---|
| frontier `claude-opus` 0.60 in / 3.0 out, ±100%, "Jegham (o3: 7.03 Wh/short)" | | Jegham has **no Opus model**. o3 short is 1.177 Wh in v6 (7.03 is from an earlier version). Nearest evidence: Sonnet-class 2.9–3.3 Wh/1K out. | **Misattributed.** 3.0 is a plausible floor (Opus ≥ Sonnet), unsourced. Tier should say "assumed". |
| large `claude-fable`, `claude-sonnet` 0.30 / 1.5, ±50%, "mdodkins, Jegham (Sonnet 3.7: 0.84 Wh/short)" | | Jegham v6: Sonnet 3.7 short = **0.950**, fit 0.135 / **2.88**. 3.5 Sonnet 0.28 / 3.34. mdodkins: 0.39 / 1.95 (unsourced). No Fable/5.x measurement anywhere. | **Output under by ~2×** against Jegham; input within range. Fable is assumed. |
| large `gpt-4` (catches 4.1, 4-turbo) 0.30 / 1.5 | | GPT-4o 0.12 / 1.12 ✓; GPT-4.1 3.1 (2× under); GPT-4 Turbo 5.8 (4× under). | Within range for 4o only. |
| medium `claude-haiku` 0.10 / 0.5, ±50%, "Jegham (4o-mini: 0.42 Wh/short)" | | Jegham v6 gives Haiku 3.5 = 0.975 short, fit 4.19 out. But Jegham assigns models with undisclosed size to the **Large hardware class (8 GPUs)** (§4.3, p. 5) and infers energy from API latency, so a small model served at low TPS looks expensive. 4o-mini short is 0.577 in v6, not 0.42. | **Unsupported either way.** 0.5 is an analogy to OpenAI's mini class, not a Haiku measurement. Should be "assumed", ±100%. |
| medium `gpt-4o-mini`, `gpt-4.1-mini` 0.10 / 0.5 | | Jegham v6: 1.87 and 1.56 Wh/1K out (4o-mini on A100 per §4.3.1). | **Under 3–4×.** |
| small `gpt-4.1-nano` 0.03 / 0.15, "Jegham (nano: 0.10 Wh/short)" | | v6: 0.207 short, 0.59/1K out. | **Under 4×**; quoted number is stale. |
| small `gemini-flash` 0.03 / 0.15, "IEA 2025 (Gemini: 0.24 Wh/query)" | | 0.24 Wh is Google's median Gemini Apps prompt (Elsworth et al. 2025), token counts undisclosed. Cannot yield a per-token figure. | **Unsourced as per-token.** Keep as assumption, say so. |
| reasoning `o3`, `o1` 0.60 / 5.0, ±150% | | o3 fit 0.54 / 4.53; o1 4.41. | **Matches.** Note text stale (o3 short 1.177, o1 2.268 in v6). |
| `gpt-5` "large (assumed)" 0.30 / 1.5, ±100%, "no published measurement" | | **Jegham v6 §7 measures GPT-5** (Fig. 6): minimal 0.41 / 1.93, high 0.67 / 18.0 per *visible* output token. Codex logs count `output_tokens + reasoning_output_tokens` as output (`codex.rs:191`), so per counted token the minimal-reasoning fit is the closer anchor; reasoning effort then shows up as more tokens, not a higher coefficient. | **Claim "no published measurement" is now false.** Suggest 0.4 / 2.0, ±100%, source Jegham v6 §7. |
| `gpt-6` "large (assumed)" | | Nothing published. | Assumption, correctly labelled. |
| unknown 0.20 / 1.0, ±200% | | – | Judgment; fine. |
| cache create = 1.25 × input (`energy.rs:103`) | | **No paper in refs measures prompt-cache writes.** 1.25× is Anthropic's cache-write price multiplier. mdodkins reproduces the same ratio without saying why. | **Unsourced; a price ratio.** |
| cache read = 0.1 × input (`energy.rs:104`) | | **No paper in refs measures prompt-cache reads.** 0.1× is Anthropic's cache-read price multiplier; OpenAI prices the same mechanism between 0.1× and 0.5× depending on model. TokenPowerBench shows energy per token rising ~3× from 2K to 10K context for a 70B model (Fig. 3, p. 6), which is the physical reason cached context is not free, but it measured uncached prefill, not cache hits. | **Unsourced; a price ratio. The single largest lever in every number conscience prints (§2).** |
| "output ≈ 5× input (API level)", spec p. 36 | | 5× is Anthropic's price ratio. Jegham's own data fit gives 9–24× for non-reasoning models. Luccioni and TokenPowerBench state no per-token ratio. | Pricing, not energy. Evidence says 10–20×. |
| uncertainty ±50 / 100 / 150 / 200% | | Not from any paper. Jegham's reported std devs are ±4–25% for Claude models, but its structural uncertainty is larger: batch size 4→8→16 changes energy ~×1.8 / ×0.57 (Appendix A, p. 18); hardware class is inferred. | **Analyst-assigned.** Defensible as bands, but the docs should say so rather than imply they come from the papers. |
| grid 0.42 kg CO2/kWh default (`manifest.rs:226`), "US average, location-based" | | Not in any paper. Jegham Table 1: AWS (Anthropic, Meta) **0.287**, Azure (OpenAI) **0.35**, DeepSeek/China 0.6. Li Fig. 2: US eGRID regions 0.1–0.8. Current EPA eGRID US average is ~0.39. | **Unsourced; high for Claude by ~45%.** Provider-specific values exist in Jegham. |
| water 1.8 L/kWh default (`manifest.rs:227`), "on-site cooling + off-site generation (Li et al. 2023)" | | Li Table 1 (p. 5), US average: on-site WUE **0.55** L/kWh, off-site EWIF **3.142** L/kWh, PUE 1.17 → **4.2 L per kWh of server energy, 3.6 L per kWh of facility energy**. Jegham Table 1 + Eq. 4: AWS 0.18/1.14 + 5.11 = **5.3**; Azure 0.30/1.12 + 4.35 = **4.6** L per facility kWh. Meta reports 3.7 scope-2. | **1.8 is unsourced and 2–3× too low for the scope the wiki claims.** (Withdrawal, not consumption, would be ~44 L/kWh; consumption is the right measure.) |
| PUE | Not applied. | Jegham's per-query Wh include PUE 1.12–1.14. Li assumes 1.1–1.17. | Consistent as long as the tier numbers are read as facility-level. Docs don't say. |

### Comparison constants (`comparisons.rs:58-135`)

| Constant | Sources in refs | Verdict |
|---|---|---|
| phone charge 15 Wh | Luccioni p. 4 fn 4: EPA **22 Wh** (0.022 kWh; 12 Wh before Jan 2024). Jegham p. 10: two phones ≈ 10 Wh, i.e. **5 Wh** each. | Sources disagree 4×; 15 is in between. Cite EPA 22 or keep 15 with the spread noted. |
| US household 29.6 kWh/day, EIA 10,791 kWh/yr | Jegham/gist use ~28 kWh/day. | ✓ |
| car mile 404 g CO2 | Luccioni cites the same EPA calculator. | ✓ |
| laptop 60 W, EV 0.28 kWh/mi, shower 65 L, EU household, bottle | Not in refs. | Reasonable round numbers; sources named in output. |
| (unused) Google search 0.30 Wh, Jegham p. 10 | | A good small-scale energy comparison to add; note it is a 2009 Google figure. |

## 2. What the cache-read coefficient does to real numbers

`conscience report energy --all --days 30 --json` on this machine, today:

| Component | Share of 261 kWh |
|---|---|
| input (uncached) | 0.1% |
| output | 16.7% |
| cache create | 24.4% |
| cache read | **58.8%** |

Tokens: 3.6 billion cache reads vs 26 M output. Sensitivity of the total to
the cache-read multiplier alone: 0.03× → 154 kWh; 0.1× → 261 kWh; 0.3× → 568
kWh; 1.0× → 1,641 kWh. Every comparison line, CO2 and water figure, the
digest headline, the `ai_token_consumption_high` signal and the history
reading "energy per output token" multiply through this one unmeasured
constant.

A second dependency: both cache coefficients hang off the **input**
coefficient (`build_coefficients`). Adopting Jegham's fitted Sonnet input
(0.135 instead of 0.30) would halve the 83% cache-derived share while the
output fix (2.88 instead of 1.5) would nearly double the 17% output share;
net, the 30-day total would move from ~261 to ~190 kWh. That the estimate
falls when better data is used is itself a sign the structure is fragile: the
cache term is anchored to the wrong quantity. Physically, a cache hit skips
prefill compute but every subsequent output token still attends over the
cached context, so cache-read cost should scale with context × output, not
with input.

## 3. Methodological issues

1. **Per-token linear model vs. Jegham's time-based model.** Jegham's fits
   show a fixed per-query overhead of 0.1–1.3 Wh (the intercept). Claude
   Code makes many short calls (a tool call is one API request with tens of
   output tokens); that overhead is not modelled. Direction: under-estimate.
2. **Cache reads and writes are priced, not measured.** No paper in refs
   covers prompt caching. The multipliers are Anthropic's price sheet.
3. **Uncertainty is symmetric; the evidence is skewed upward.** Measured
   Sonnet-class output cost is ~2× the code's; per-call overhead is omitted;
   Luccioni's unbatched A100 figures (BLOOMz-7B: 0.104 kWh per 1,000
   inferences of ~10 tokens ≈ 10 Wh/1K output, Table 3 p. 7) and
   TokenPowerBench's Llama 3 405B on 16 H100s (116–235 J/token ≈ 32–65
   Wh/1K, Fig. 5 p. 6) show that the tier values assume hyperscale batching.
   Only batching (Jegham App. A: ×0.55 per doubling from 4 to 16) pulls the
   other way.
4. **Claims attributed to Luccioni and TokenPowerBench are not in them.**
   Luccioni reports per-task kWh per 1,000 inferences and a 15× gap between
   text generation and masked LM; TokenPowerBench reports prefill vs decode
   energy, batch, context and quantization effects. Neither states a
   10–30× output:input per-token ratio. The ratio *is* supported by
   regression on Jegham's own table (9–24×).
5. **Stale version.** Every Jegham number in source notes and INDEX is from
   an earlier arXiv version; the PDF in refs is v6 and disagrees with all of
   them except GPT-4o.
6. **Water scope claim doesn't match the constant.** The wiki says the water
   figure follows Li's scope-1 + scope-2; Li's own US-average numbers for
   that scope are 3.6–4.2 L/kWh, not 1.8.
7. **Extended thinking is handled correctly** (Claude bills thinking as
   output tokens; Codex adds reasoning tokens to output), so the "reasoning"
   tier's higher coefficient is only right for o1/o3, whose visible-output
   figures in Jegham include hidden reasoning time. Good as is.
8. **Water conversion.** `water = total_wh/1000 × L/kWh` applies one factor
   to facility energy; Jegham's Eq. 4 splits on-site (÷PUE) and off-site.
   With a combined factor per facility kWh this is fine.

## 4. What the papers support that the code does not use

- **Per-provider carbon and water intensity** (Jegham Table 1): AWS 0.287
  kg/kWh, Azure 0.35; water 5.3 vs 4.6 L/kWh. Model prefix already implies
  the provider.
- **Reasoning effort as a cost driver** (Jegham §7): 0.67 → 33.8 Wh per query
  minimal → high on GPT-5. Codex logs carry `reasoning_output_tokens`
  separately; a "reasoning share" line would be an observation, not an
  estimate.
- **Context length raises per-output-token energy** (TokenPowerBench Fig. 3):
  ~3× from 2K to 10K for 70B. This is the honest basis for a cache-read term
  and argues for a context × output formulation.
- **Batch-size sensitivity** (Jegham App. A) as the stated reason for the
  band width.
- **Per-query overhead** (Jegham intercepts) as a per-call term.
- **Water consumption vs withdrawal** (Li §2.1): conscience reports
  consumption; the docs should say so, since withdrawal is ~10× larger.
- **Location-based vs market-based** (Li fn 3, Jegham §4.4 Scope 2 only):
  already stated in the wiki. ✓

## 5. Concrete proposals

| # | Change | Paper-backed value | Effort |
|---|---|---|---|
| P1 | Fix the library: re-download Luccioni (arXiv:2311.16863) under its name; rename the current `de_Vries…` file to Luccioni; add de Vries (Joule, doi:10.1016/j.joule.2023.09.004) or drop the INDEX entry; delete the space-physics PDF; correct INDEX IDs; replace "Patterson 2025 Joule 0.31 Wh" with Elsworth et al. 2025 arXiv:2508.15734, 0.24 Wh; credit 0.24 to Google and 0.34 to Altman, not IEA; note Jegham v6 and TokenPowerBench 2025. | – | S |
| P2 | Update every Jegham number in source notes and INDEX to v6 Table 4. | see §1 | S |
| P3 | Claude Sonnet/Fable tier: 0.15 in / 2.9 out, ±50%, source "Jegham v6 fit, Sonnet 3.7". Fable: same values, tier name "large (assumed Sonnet-class)". | Sonnet 3.7 fit | S |
| P4 | Opus: keep 0.60 / 3.0 but rename tier "frontier (assumed)", note "no published Opus measurement; ≥ Sonnet". Haiku: keep 0.10 / 0.5, rename "medium (assumed)", ±100%, note why Jegham's Haiku figure is not usable. | – | S |
| P5 | OpenAI: GPT-4.1 3.1, 4.1-mini 1.6, 4.1-nano 0.6, 4o-mini 1.9, 4o 1.1 Wh/1K out; GPT-5 0.4 / 2.0 ±100% with source "Jegham v6 §7, minimal reasoning; effort raises tokens, not the coefficient". GPT-6 stays assumed. | Table 4, Fig. 6 | S |
| P6 | Cache read/write: keep 0.1× / 1.25× but (a) change source note to "Anthropic price ratio; unmeasured"; (b) carry a separate ×⅓–×3 band on the cache-read component and fold it into the total range; (c) print one line under the table: "N% of this estimate rests on the cache-read coefficient, which no published study has measured." Same line in the digest and the signal evidence. | TokenPowerBench Fig. 3 for the *existence* of the cost | M |
| P7 | Water default 1.8 → 4.6 L/kWh (Jegham Azure/AWS mean; Li US avg 3.6 as the low end), source note "Li 2023 Table 1; Jegham 2025 Table 1, Eq. 4; consumption not withdrawal". | Li p. 5, Jegham p. 4/7 | S |
| P8 | Grid default 0.42 → per provider from model prefix: claude-* 0.287 (AWS), gpt-*/o* 0.35 (Azure), else 0.39 (US average, EPA eGRID), all overridable as now. | Jegham Table 1 | S |
| P9 | Docs: uncertainty bands are analyst-assigned; figures are facility-level (PUE included); batch size is the largest known swing (Jegham App. A); Luccioni/TokenPowerBench citations reworded to what they show. | – | S |
| P10 | Optional per-call overhead term (Wh per API request, ~0.1–0.3 for non-reasoning models from Jegham intercepts). Needs request counts per session; Claude logs have them. | Jegham fits | M |
| P11 | Optional: `report energy` shows reasoning-token share for Codex sessions (observation) and links it to Jegham §7. | Jegham Fig. 6 | S |

Things where the honest answer is "we cannot know for closed models; say
so": Opus, Haiku, Fable 5.x, GPT-6, Gemini Flash per-token, and every cache
coefficient.

## 6. Priority

1. **P6 (cache-read transparency and band).** 59% of the current estimate,
   0% measured. Until it is labelled, every downstream number overstates its
   confidence.
2. **P3 + P5 + P2 (bring tiers to Jegham v6).** Sonnet-class output is 2×
   low, GPT-5 has a measurement now, mini/nano are 3–4× low, and all quoted
   figures are stale.
3. **P7 + P8 + P1 (water, grid, library).** Water is 2–3× low against the
   very paper it cites; grid is ~45% high for Claude; two of five PDFs are
   the wrong paper and two web sources are mis-cited.

Sources used: Jegham et al. 2025 v6 (refs), Luccioni et al. 2024 (refs, under
the de Vries filename), Li et al. 2023 v5 (refs), Niu et al. TokenPowerBench
2025 (refs), Elsworth et al. 2025 arXiv:2508.15734 (abstract, web), de Vries
2023 Joule (abstract and quoted text, web), mdodkins gist (web).
