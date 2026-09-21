# Reference impact analysis: labor, governance, commons and decolonial references vs. conscience 0.7.0

Date: 2026-09-21. Read against `src/ethics/{models,signals,multi,reflection,manifest}.rs`,
`conscience.example.yaml`, and `refs/INDEX.md`. No code changes. Page numbers are
PDF pages of the files in `refs/`. Companion document: `2026-09-21-energy-coefficient-verification.md`.

What conscience has today, for reference: seven principles (Human Agency, Equity of
Benefit, Transparency, Developer Growth, Environmental Cost, Code Provenance,
Security), 33 signal ids (signals.rs, multi.rs), eight reflection questions
(reflection.rs), and a manifest with mission, beneficiaries, cost bearers, team
size and senior/mid/junior counts, learning goals, monthly review, thresholds.

---

## Part A. Labor

### A1. Key claims

**ILO, "Work Transformed" (June 2025, research brief for the Doha conference).**
- Definition used throughout (Table 1, p. 10): "Automation occurs when technology
  performs certain tasks of an occupation, whereas augmentation happens when
  technology improves human capabilities without replacing them." Outcomes hinge on
  exposure × complementarity (p. 4, after Cazzaniga et al. 2024).
- Empirical: 2.3% of jobs globally have high automation potential; 0.4% in low-income
  vs 5.1% in high-income countries; clerical support 24% of tasks highly exposed (p. 2).
  10.4% / 13.4% of jobs could benefit from augmentation (p. 3). "To date, the effect
  of AI on overall employment levels remains limited... companies are adjusting through
  hiring freezes and voluntary attrition" rather than layoffs (p. 2–3).
- Developer-specific evidence it cites: Peng et al. 2023, tasks completed 55.8% faster
  with an AI coding assistant (p. 3); Brynjolfsson et al., +14% for support agents,
  +34% for inexperienced workers (p. 3).
- Recommendation-grade: "over-reliance on automation can lead to skill degradation"
  (p. 9, citing IMF 2024); algorithmic management "track[s] worker behaviour, evaluate[s]
  performance, and allocate[s] tasks, often leading to intensified workloads" and is
  "deployed in hiring, performance management, and scheduling without consultation"
  (p. 6, p. 9); worker voice "is not a 'nice to have'" (p. 7).

**ILO, "Workers' exposure to AI: what indicators tell us and what they don't" (Feb 2026).**
- "Exposure indicators reveal technological susceptibility, not labour market
  outcomes" and "should not be used to forecast job losses" (p. 1, p. 8).
- Newer capability-based measures put computing and mathematical occupations among
  the most exposed; correlation of exposure with wages is positive, 0.27–0.54 (Table 2,
  p. 6). The example task weight for software developers is "complex problem solving"
  (p. 2).
- Five structural limits of every exposure measure (p. 7–8): static task lists, partial
  view (no productivity or demand effects), adoption dynamics, subjectivity, US-centric
  data. Also: indirect exposure spreads through occupational networks (p. 6–7).

**Stanford HAI AI Index 2026 (April 2026, 425 pp.).**
- Takeaway 9 (p. 11) and Chapter 4 takeaway 7 (p. 173): "U.S. developers ages 22 to 25
  saw employment fall nearly 20% from 2024, even as the headcount for older developers
  continues to grow." The chapter body (p. 221) says instead that employment for the
  youngest workers "has declined since 2022" and "by September 2025... had fallen close
  to 20% from its 2022 peak." The report's own baseline is inconsistent.
- Underlying studies (table, p. 221): Brynjolfsson et al. 2025, ADP payroll, "Canaries
  in the coal mine": −15% to −16% employment for early-career workers in exposed fields;
  Hosseini Maasoum and Lichtinger 2025, 62 M workers / 285 K firms, "seniority-biased
  technological change", decline "driven by slower hiring".
- Productivity (p. 219–220): Copilot users completed 26% more pull requests (Cui et al.
  2025), with "less experienced workers tend[ing] to benefit the most"; but METR found
  "experienced open-source developers became 19 percent slower when using AI assistance,
  with a disconnect between how helpful the developers thought the tools were and how
  they actually performed" (Becker et al. 2025).
- One-third of organizations expect AI to reduce their workforce in the coming year
  (p. 173). CS undergraduate enrollment fell 11% between 2024 and 2025 (p. 292).

**Index claims not supported by the documents ("don't believe the hype").**
- `refs/INDEX.md` says the Stanford report "documents... Junior employment drops 9-10%
  within six quarters of AI tool adoption." That figure appears nowhere in the report
  text. It may come from the Hosseini & Lichtinger paper, which conscience does not
  hold; it cannot be cited to the AI Index.
- The "~20% since 2024" phrasing copies the report's takeaway; the chapter body and
  the source study measure from late 2022. Cite "from its 2022 peak, by September 2025".
- INDEX calls this "measured fact" that "transforms MH's de-skilling warning". The
  studies measure hiring of a cohort, not de-skilling of anyone; ILO 2026 warns
  precisely against reading exposure-type data as displacement.

### A2. What conscience already covers

| Reference claim | Conscience today |
|---|---|
| Over-reliance / skill degradation (ILO 2025 p. 9; NIST automation bias) | `ai_turn_ratio_high/moderate/balanced` (signals.rs:396–424, Human Agency); `agent_approval_near_automatic` (signals.rs:942); reflection "Are team members learning new skills... or becoming more dependent" (reflection.rs:86) |
| Junior developers | `manifest_juniors_no_learning_goals` (signals.rs:180) is the only junior-aware signal; `team.roles.{senior,mid,junior}` in manifest.rs:93 |
| Velocity claims | `github_velocity_baseline` (signals.rs:363) reports counts without a speed claim |
| Worker voice / social dialogue | `reflect --interactive`, `--save`, `retro` are exactly a voice mechanism; "no surveillance tool" in CLAUDE.md |
| Algorithmic management warning | Partly honored: signals are project-level. Edge cases: `report authorship` (per contributor) and `report attention` (per machine, so per person on a laptop) |
| Exposure vs outcome | The `Signal` doc comment says "Signals never render verdicts" (models.rs:131); the ILO 2026 framing is the same idea, better stated |

### A3. Gaps

1. **Automation vs augmentation is not operationalized.** The turn ratio counts
   exchanges, not who is doing the thinking. Nothing distinguishes a session where the
   human directs and the tool executes from one where the human only says "yes".
2. **No junior-pipeline reading.** The manifest holds junior counts but nothing tracks
   them across snapshots or asks about hiring. The strongest labor evidence in refs is
   about hiring, not tasks.
3. **No perception check on speed.** METR's 19% slower / felt faster result is the
   single most relevant "hype" finding for this tool's users, and nothing surfaces it.
4. **Signals are not framed as exposure-like indicators** in the wiki or output; users
   can read `ai_turn_ratio_high` as "this developer is dependent".
5. **Per-person outputs** (authorship, attention) sit on the ILO's algorithmic-management
   line without saying so.

### A4. Proposals

| # | Change | Data source | Honesty check | Effort |
|---|---|---|---|---|
| L1 | Correct INDEX: drop the "9–10% within six quarters" claim or source it to the Hosseini & Lichtinger paper and add that PDF; cite the decline as "close to 20% from its 2022 peak by September 2025 (p. 221)"; cite Brynjolfsson −15/−16%. | – | Fixes a misattribution. | S |
| L2 | New Developer Growth reflection question (only when AI data exists): *"Have we measured whether AI makes us faster here, or do we assume it? Stanford's 2026 Index reports Copilot users completing 26% more PRs, and experienced developers 19% slower while believing they were faster. Which are we?"* Data context: PRs merged, sessions, turn ratio. | GitHub + logs | Question, not a finding. The tool cannot measure speedup. | S |
| L3 | New Equity of Benefit reflection question when `team.roles.junior == 0` and `team.size >= 3`: *"This team has no junior developers. Employment for developers aged 22–25 fell about 20% from its 2022 peak while older cohorts grew (AI Index 2026, p. 221). Is that a choice here? What would it take to bring someone in and let them learn?"* | manifest | Self-reported roles; a question avoids the tokenism of a "no juniors" warning. | S |
| L4 | Record `team.roles.*` and `team.size` as manifest metrics in the snapshot so `report history` shows whether juniors are joining or leaving. | manifest → snapshot | Self-reported; labelled Observation of the manifest, not of reality. | S–M |
| L5 | Augmentation proxy, project-level: median human message length and share of human turns that are short acknowledgements ("yes", "ok", "continue", "go ahead") per session, reported as an Info signal *"Human turns are mostly approvals"* when the share exceeds a threshold and session count is enough. | Claude Code / Codex logs (human message text, not stored) | A proxy for automation vs augmentation; gameable; must aggregate per project and never per person. Say "mostly approvals" not "dependent". | M |
| L6 | Wiki (The Seven Principles) and the examine preamble: signals are exposure-like indicators in ILO's sense: what the tool did, not what happened to people; they "cannot be interpreted as predictions of job displacement, productivity gains or reskilling needs" (ILO 2026 p. 1). | – | Docs. | S |
| L7 | `report authorship` and `report attention`: add one line naming the algorithmic-management risk (ILO 2025 p. 9) and that these views are for the person themself or a team that asked for them. | – | Docs/output. | S |

### A5. Tensions

- **Measuring junior growth is individual monitoring.** Any "growth trajectory" metric
  needs per-person longitudinal data, which is what ILO 2025 (p. 9) and MH 150 warn
  against. Keep growth to self-report (learning goals, reflections) and team-level
  counts. Do not build L4 into a per-person view.
- **Exposure misread as outcome.** ILO 2026's central warning applies to conscience's
  own signals; L6 is the mitigation.
- **The evidence cuts both ways.** ILO 2025 leans optimistic (augmentation outweighs
  automation, ATM-teller analogy); Stanford's hiring data is the counterweight.
  Conscience should present both rather than adopt the de-skilling framing as settled.

---

## Part B. Governance

### B1. Structure of each framework

**NIST AI 600-1, Generative AI Profile (July 2024).** Twelve risks "unique to or
exacerbated by GAI" (§2.1–2.12, pp. 5–12): CBRN information; Confabulation; Dangerous,
violent or hateful content; Data privacy; Environmental impacts; Harmful bias and
homogenization; Human-AI configuration; Information integrity; Information security;
Intellectual property; Obscene content; Value chain and component integration. Then
212 suggested actions keyed to the AI RMF functions (GV-, MP-, MS-, MG-), each tagged
with the risks it addresses (§3, pp. 12–52). Notable for conscience:
- §2.5: "Currently there is no agreed upon method to estimate environmental impacts
  from GAI" (p. 8). MS-2.12-002 "Measure or estimate environmental impacts (e.g., energy
  and water consumption)"; MS-2.12-003 distinguish inference from training resources;
  MS-2.12-004 "address green-washing concerns".
- §2.7: automation bias, "excessive deference to automated systems" (p. 9).
- §2.9: prompt injection named as GAI attack surface (p. 10). MS-2.7-004 track
  "unauthorized access attempts, inference, bypass, extraction, penetrations, or
  provenance verification".
- §2.10: memorized training data may infringe copyright; legal status "being debated"
  (p. 11). MS-2.10-001 covers "licensed, patented, personal, proprietary... information".
- §2.12: third-party components "improperly obtained or not properly vetted" (p. 12);
  MG-3.1-001 organizational risk tolerance for third-party models and datasets;
  MP-2.2-001 provenance where a component is an upstream dependency.

**Responsible AI Pattern Catalogue (Lu et al., ACM Computing Surveys 2024).** 63
patterns in three tiers: governance (24: industry-, organization-, team-level, Fig. 4
p. 8), process (17 across requirements, design, implementation, testing, operation,
Fig. 5 p. 20), product (22 across supply chain, system, operation infrastructure,
Figs. 6–7 pp. 27–29). Bridges principles to engineering practice; explicitly notes
principles are "too abstract to be put into practice" without patterns.

### B2. Mapping: every conscience signal → NIST risk → RAI pattern

| Signal id | Principle | NIST risk (§) | Example NIST action | RAI pattern |
|---|---|---|---|---|
| ai_turn_ratio_high / moderate / balanced | Human Agency | 2.7 Human-AI configuration | GV-3.2-003 policies for human-AI configurations | AI mode switcher (5.2.1); Continuous ethical validator (5.3.1) |
| agent_approval_near_automatic | Human Agency | 2.7 | MG-2.2-006 auditing of AI-generated content | AI mode switcher |
| agent_actions_denied | Security | 2.7 | GV-3.2-004 recourse mechanisms | Ethical blackbox (log) |
| github_review_engagement_low, github_merge_time_fast | Human Agency | 2.7 (automation bias in review) | – | Ethical acceptance testing (4.4.1) |
| ai_agent_orchestration_heavy, multi_agent_orchestration_highest | Human Agency | 2.7 | – | AI mode switcher |
| automation_failing_repeatedly | Human Agency | 2.7; 2.12 | MG-4.1-003 monitor content performance | Continuous ethical validator |
| security_prompt_injection_pr | Security | 2.9 Information security | MS-2.7-004 | Ethical sandbox (5.x) |
| security_sensitive_file_read / write, security_credential_access | Security | 2.4 Data privacy; 2.9 | MP-4.1-003 apply IT governance to GAI use | Ethical blackbox |
| security_network_exfiltration, security_encoding_obfuscation | Security | 2.9 | MS-2.7-004 | Ethical blackbox |
| ai_session_length_extreme, ai_tokens_per_turn_high | Security | 2.7; 2.9 | – | – |
| multi_security_warnings | Security | 2.9 | – | Global view auditor (5.3.x) |
| ai_new_file_ratio_high | Code Provenance | 2.10 Intellectual property; 2.12 Value chain | MS-2.10-001 | Bill of materials registry (5.1.1); Co-versioning registry (5.1.3) |
| ai_token_consumption_high, ai_cache_efficiency_good, multi_token_concentration | Environmental Cost | 2.5 Environmental impacts | MS-2.12-002/003/004 | Standardized reporting (3.2.5) |
| ai_bash_volume_high | Transparency | 2.7 | – | Ethical blackbox |
| github_single_contributor, github_contribution_concentration_* , github_contributions_distributed | Equity of Benefit | none (NIST has no team-equity risk) | – | Diverse team (3.3.3) |
| github_velocity_baseline | Transparency | none | – | Standardized reporting |
| manifest_review_stale | Transparency | none directly (GV-1.x governance cadence) | GV-1.3-001 risk documentation | Extensible, adaptive and dynamic ethical risk assessment (4.5.2); Continuous documentation using templates (3.3.5) |
| manifest_no_beneficiaries, manifest_value_documented, manifest_ai_sentiment | Equity / Transparency | none | – | Stakeholder engagement (3.3.4); Ethical user story (4.1.4) |
| manifest_juniors_no_learning_goals | Developer Growth | none (NIST has no workforce-development risk) | – | Ethics training (3.2.8), loosely |
| multi_ai_turn_ratio_highest | Human Agency | 2.7 | – | Global view auditor |

Commands as patterns: `conscience.yaml` = Continuous documentation using templates +
Ethical user story; `reflect`/`retro` = Stakeholder engagement (team-internal);
snapshots = Ethical blackbox (the immutable record); `report history` = Continuous
ethical validator; `examine --pr --comment` = an Ethical acceptance test at the PR gate;
`prune` = AI mode switcher (a kill switch for unattended automation);
`manifest_review_stale` = the dynamic ethical risk assessment cadence.

**NIST risks and RAI patterns with no conscience counterpart** (relevant to AI-assisted
coding):
- 2.2 Confabulation: no correctness proxy (CI pass rates, reverts, bug-fix commits).
- 2.6 Harmful bias and homogenization: nothing; for code this is "everyone's code
  looks the same" and is barely measurable.
- 2.12 Value chain: dependencies AI sessions added are invisible. RAI Bill of
  materials registry / RAI software bill of materials (3.2.7).
- 2.10 IP: no view of license headers or licenses of new files.
- RAI Role-level accountability contract (3.2.6): who signs for AI-written code.
- RAI Verifiable claim for AI system artifacts (3.3.8): a signed statement of AI
  involvement per PR; conscience's PR comment is close but is a report, not a claim.

### B3. Gaps, by measurability

Measurable from git, logs, or GitHub:
- Dependencies added in AI-touched commits (diff of Cargo.toml, package.json,
  go.mod, requirements.txt, pyproject) → an observation "AI sessions added N
  dependencies this interval" (2.12; RAI 3.2.7 / 5.1.1).
- New files without the repository's license header when LICENSE is Apache/MPL-style
  (2.10). Narrow but exact.
- CI status per PR (already fetched? PR data is; check runs are not) → confabulation
  proxy. Larger.

Requires human attestation:
- Who is accountable for AI-written code (role-level accountability).
- Whether third-party model use is consistent with organizational risk tolerance
  (MG-3.1-001) — a `conscience.yaml` field, at most.

### B4. Proposals

| # | Change | Data source | Honesty check | Effort |
|---|---|---|---|---|
| G1 | A wiki page "Crosswalk to NIST AI 600-1 and the RAI Pattern Catalogue" containing the table above, and an optional `refs` field on each signal in `--json` and the snapshot (`{"nist": ["2.7"], "rai": ["5.2.1"]}`), left out of the terminal output. | – | A reference for compliance readers; does not change how signals are chosen or shown. | S–M |
| G2 | Do **not** restructure the scorecard by NIST function or add a "compliance report" mode. See B5. | – | – | – |
| G3 | New Code Provenance observation: dependencies added in AI-touched commits, with the manifest files that changed. Info severity. | git + logs (files touched) | An observation; whether the dependency is vetted is a question. | M |
| G4 | New Code Provenance observation (only when the repo's LICENSE expects headers): AI-written new files missing the header. | git | Exact but narrow; skip when no header convention exists. | S |
| G5 | `conscience.yaml`: optional `team.accountable_for_ai_code: [names or roles]`; surfaced in the Code Provenance reflection data context. | manifest | Attestation; the tool cannot verify it. | S |
| G6 | Environmental caveat text cites NIST §2.5 "no agreed upon method" and MS-2.12-004 green-washing, alongside the existing sources. | – | Docs. | S |
| G7 | Later: CI check-run outcomes per PR as a confabulation proxy (2.2), feeding a "reverts and failed checks after AI-heavy PRs" reading in history. | GitHub API | Causality is not available; report as correlation. | L |

### B5. Tensions

- **NIST is written for organizations deploying GAI systems to the public.** Of the
  twelve risks, CBRN, obscene content, dangerous content and most of information
  integrity have nothing to do with a team using a coding assistant. Reorganizing
  conscience around the taxonomy would foreground empty categories and bury the
  encyclical's questions (dignity, common good, subsidiarity, solidarity) that NIST does
  not ask at all: there is no NIST risk for equity within a team, for juniors, or for
  who benefits.
- **Compliance vocabulary changes behavior.** A team that sees "MS-2.7-004 satisfied"
  stops discussing; a team that sees "AI read credential directories 3 times; what do
  we think about that?" talks. Conscience's value is the second. The crosswalk (G1)
  gives the first audience a way in without changing the tool.
- **Where NIST helps:** it names automation bias precisely (§2.7), it says the
  environmental estimate problem is unsolved (§2.5, useful backing for conscience's
  caveats), and its value-chain category exposes a real blind spot (G3).
- **Subsidiarity:** the RAI catalogue's team-level patterns (diverse team,
  stakeholder engagement, continuous documentation) are the tier that matches
  "teams own their own evaluation"; the industry-level ones (certification, trust
  marks) are not conscience's business.

---

## Part C. Commons and decolonial

### C1. Key claims

**Open Future / Bertelsmann, "Public AI – White Paper" (May 2025).**
- Argument: AI is being built as private infrastructure; "Public AI" is the
  countervision, assessed on a six-level "gradient of publicness" (ch. 4, pp. 52–55).
- Governance principles include **Reciprocity**: "Public and private actors that
  benefit from public AI resources should ensure that downstream applications and
  derivative products adhere to these governance principles. This helps prevent the
  privatization of public value and protects against corporate capture." (p. 57) and
  **Sustainable AI development**: "fair resource use", sustainable compute procurement
  (p. 57).
- Data pathway (pp. 65–67): current practice "oscillate[s] between proprietary control
  and unrestrained extraction from public sources"; commercial training on "the
  entirety of the public internet... results in extraction of value from global
  knowledge and culture commons"; Data Provenance Initiative shows consent for crawling
  "steadily decreasing"; Wikimedia reports crawler traffic "becoming a financial
  burden" (p. 65). The harm: "private actors capture the economic value generated by
  data without giving back to the people and institutions that created or maintained
  it as a public good" (p. 66). Free-riding named at the data layer (p. 59).
- Evidence vs argument: the extraction and consent trends cite third-party data; the
  reciprocity principle is a policy proposal. The "data winter" phrase is Verhulst 2024,
  cited in footnote 181 (p. 65), not Open Future's own finding as INDEX implies.

**Creative Commons, "Guidance on CC Licensing in the Age of AI" (27 Aug 2026).**
- CC licenses "are copyright tools, and they only apply where copyright applies";
  many AI uses fall under exceptions, so restrictive CC licenses "do little to prevent
  AI usage" while limiting human reuse (pp. 3, 7–8).
- Licensors are retreating to restrictive terms "as a direct reaction to AI ingestion
  of published works that is often done without appropriate attribution or
  reciprocity"; "the retreat toward restrictions has real costs" (p. 8).
- CC Signals: a framework "for communicating expectations around AI use of content or
  data" (p. 12).
- **Scope limit INDEX misses:** "This document considers only the licensing and
  sharing of human-generated (not wholly AI-generated) content and its use in AI"
  (p. 5). It says nothing about attribution of AI outputs or AI-generated code. INDEX's
  "operationalizes the attribution that Leiden calls for" and "practical mechanisms for
  code provenance... in AI contexts" overstate it.

**Nemorin & Bonami, "AI ethics through a decolonial lens" (AI & Society 2026).**
- Mainstream AI ethics is "predominantly grounded in Western epistemological
  traditions", principled/deontological, with "limited reinforcement mechanisms, often
  deployed for marketing" (pp. 6445–6446). Three reorientations (p. 6445): (1) AI to
  redress power asymmetries; (2) local data sovereignty and equitable epistemic
  participation; (3) AI as relationally entangled with humans, more-than-humans and
  the Earth.
- Decolonial computing asks "who is doing computing, where they are doing it" (Ali
  2016, quoted p. 6450). Environmental costs "are most often felt in Global South
  geographies" (p. 6452); "the Global South, despite holding 85% of the world's
  inhabitants, is still seen as an extraction site (both for data and natural
  resources)" (p. 6453). Calls for "a reimagining of metrics of progress" (p. 6453),
  a slower pace (Krenak, p. 6454), and open sourcing of datasets to tackle data poverty
  (p. 6454).
- Argument, not evidence: it is a literature-based essay; its factual claims are
  cited to Crawford, Kara, Hao and others, not measured here.

### C2. What conscience already covers

| Claim | Conscience today |
|---|---|
| Hidden costs, who bears them | `cost_bearers` in the manifest (manifest.rs:37, example yaml "Who bears the costs? Often invisible. MH 173"); `manifest_no_beneficiaries`; who-benefits reflection (reflection.rs:132) |
| Environmental externalities fall elsewhere | Environmental Cost principle; energy, CO2, water estimates; proportionality question (reflection.rs:223). No geography. |
| Attribution / provenance | Code Provenance principle (Leiden O4–O6) with one signal, `ai_new_file_ratio_high` (signals.rs:448); transparency question about disclosure (reflection.rs:165) |
| Data sovereignty for the team's own data | Local snapshots, allowlisted export, no transcripts stored, PR comments stripped of paths and names. This is the strongest alignment in the codebase. |
| Pluralism / subsidiarity | Thresholds and value categories are configurable; principles and questions are not. |
| Reciprocity to the commons | Nothing. |
| Whose knowledge trained the tool | Nothing. |

### C3. Gaps

(a) **Commons reciprocity.** No signal or question about what the team gives back:
upstream fixes, open licensing, sponsorship, documentation. Per-person GitHub activity
outside the org is measurable but is surveillance and gameable; project-level facts
(LICENSE present and open, FUNDING.yml, whether the repo is public) are measurable
and neutral, but thin.

(b) **Provenance of AI output in an open-source repo.** Nothing observes whether
AI-written files respect the repo's license conventions (headers, NOTICE); the CC
guidance does not cover this and NIST §2.10 says the law is unsettled. Question form.

(c) **Whose knowledge, and the framing itself.** Conscience's framework is an
encyclical and a mathematics declaration; Nemorin & Bonami would call that a Northern
normative morality. MH does contain the counterweights (solidarity, MH 173 on unseen
labor, MH 101 on energy and water), but teams in other traditions have no way to bring
their own principle or question into the tool.

(d) **Actionable decolonial questions for a software team.** Where does our compute
run and who lives next to it; do our beneficiaries include people outside the
Global North; are we producing more because we can, at a pace no one asked for
(Krenak); did the tool's training draw on communities that get nothing back.

### C4. Proposals

| # | Change | Data source | Honesty check | Effort |
|---|---|---|---|---|
| C1 | New Code Provenance reflection question, always present: *"The tools we use were trained on other people's code and writing, much of it shared under open licenses by people who did not anticipate this use. What do we give back: upstream fixes, open licensing, sponsorship, documentation? Is any of it proportionate to what we take?"* Data context: repo visibility and license if known, dependency count. | GitHub API (license, visibility) | A question. Measuring reciprocity as a score would invite tokenism (a FUNDING.yml to satisfy a signal). | S |
| C2 | Observation, Info: repository license and visibility from the GitHub API ("public, MIT") or "no LICENSE file", in the Code Provenance data context. | GitHub API / git | Fact, no judgment. | S |
| C3 | Code Provenance question when the repo is open source and AI wrote new files: *"AI-generated files landed in an openly licensed repository. Do they follow its license conventions, and would we sign our names to them as authors (Leiden O5)? The legal status of AI output is unsettled (NIST §2.10); what is our position?"* | git + logs | Question; G4 gives the narrow measurable part. | S |
| C4 | `conscience.yaml`: `reflection.questions` list letting a team add its own principle name, source, and question text; rendered alongside the seven and saved in reflections and retro. Optionally `principles.disabled` to drop one. | manifest | The concrete subsidiarity mechanism; lets a team reason in Ubuntu, CARE, or its own terms without conscience pretending to. | M |
| C5 | Environmental Cost: use the per-provider region from the energy work (analysis-energy P8) to add one line to the proportionality data context: *"Estimates assume <provider> data centres; their energy, water and grid emissions fall on the communities around them."* And add a `cost_bearers` template entry in the example yaml: "Communities near the data centres that run our AI tools". | energy config | Docs plus one line; no new measurement. | S |
| C6 | Who-benefits question gains a clause: *"...and are any of the beneficiaries outside the markets we usually design for?"* | – | Wording. | S |
| C7 | Correct INDEX: "data winter" is Verhulst 2024 as cited by Open Future; the CC guidance excludes AI-generated content and does not provide provenance mechanisms for code; note Open Future's reciprocity principle is about public AI resources and policy, and the team-level reading is conscience's extrapolation. | – | Record keeping. | S |
| C8 | Do not add a "decolonial" or "reciprocity" signal computed from repository data. | – | It would be the proceduralism the paper criticizes: a checkbox derived from a git tree. | – |

### C5. Tensions

- **CC vs. protection.** CC says restrictive licensing to deter AI is ineffective and
  harms human reuse; Open Future and the decolonial paper stress protecting commons and
  communities from extraction. Conscience must not nudge a license choice either way;
  C1 and C3 ask, they do not advise.
- **Open sourcing as remedy vs. data sovereignty.** Nemorin & Bonami both call for
  open datasets against data poverty (p. 6454) and for community control over data
  (p. 6451–6452). A team's "give back" can be the wrong gift if the recipients did not
  ask. Another reason for questions over scores.
- **Conscience's own lens.** The encyclical is a global institution's text in a Western
  philosophical tradition; Leiden is a European mathematicians' declaration. The
  decolonial critique of "universalist and procedural" ethics applies. The honest
  response is C4 (a team can bring its framework) plus saying so on the Seven Principles
  page, not adding a token principle.
- **Relational dignity vs. individual metrics.** MH's "no one is saved alone" (MH 73)
  and Ubuntu-style relational ethics point the same way as conscience's team-level
  aggregation and against per-person views; another reason to keep authorship and
  attention marked as self-views (proposal L7).

---

## Part D. Priority across all three groups

1. **Fix the record (L1, C7, and the energy library fixes).** Three INDEX claims are
   not supported by the documents they cite (the six-quarters figure, CC as a
   provenance mechanism, "data winter" attribution), the Stanford baseline is
   misquoted, and two energy PDFs are the wrong paper. A tool whose principle is
   "verify claims against evidence" cannot carry these.
2. **Questions, not scores, for the new ground (L2, L3, C1, C3, C6, plus C4 so teams
   can add their own).** The strongest new material is the METR perception gap, the
   junior hiring data, and reciprocity; all three are best delivered as reflection
   questions with data context, which is a few hours of work and no new measurement.
3. **Two measurable blind spots and one crosswalk (G3, G4, G1).** Dependencies added in
   AI sessions and missing license headers are observable, exact, and map to NIST §2.10
   and §2.12; the crosswalk page serves compliance readers without changing the tool.
   Leave L5 (approval-share proxy) and G7 (CI outcomes) for later; they are the ones
   that could tip into judgment.

What should not change: the seven principles, the scorecard's framing, the
project-level aggregation, and the rule that signals present evidence rather than
verdicts. Every document read here either supports that stance (ILO 2026, RAI
team-level patterns, Nemorin & Bonami on proceduralism) or is silent on it.
