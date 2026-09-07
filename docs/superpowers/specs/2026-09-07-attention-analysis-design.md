# Attention & Flow Analysis — Design

**Date:** 2026-09-07
**Status:** Approved design, pending implementation plan

## Purpose

Evaluate and visualize how a developer spends attention across projects when
working with AI assistants. Modern agentic work often means orchestrating
several sessions at once — one minute here, two minutes there — which looks
chaotic under traditional single-project metrics. This feature measures that
style honestly instead of penalizing it: it estimates active time per
session/project, detects context switches, infers flow episodes (which may
span multiple projects), and measures concurrent-session orchestration.

This serves the Developer Experience dimension (CLAUDE.md: "cognitive load
proxies: context-switching frequency, session length"). It is a personal
discernment tool: it analyzes the user's own local logs, locally. It is not
a surveillance mechanism, and its outputs carry explicit caveats that all
estimates are floors and hypotheses, not truths.

## Data source and feasibility

Claude Code session logs (`~/.claude/projects/*/*.jsonl`) timestamp every
event to the millisecond. Genuine human prompts are distinguishable from
machine-generated `user` events:

- Human prompt: `type: "user"`, `message.content` is a **string**,
  `isSidechain` false, no `isMeta`.
- Tool result: `type: "user"`, `message.content` is a **list** containing
  `tool_result` blocks.
- Subagent traffic: `isSidechain: true` — excluded from attention analysis.
- Hook/caveat/meta string-content events (e.g. "Caveat: the messages
  below…") are excluded by content-pattern rules; typed slash commands
  count as touchpoints (the human acted).

### Duplicate history (required dedup)

Resumed, continued, or compacted sessions can copy prior history into a new
`.jsonl` file: the same human prompts appear, with identical timestamps, in
more than one session file. Touchpoints MUST be deduplicated across files —
by message UUID where present, falling back to identical
(timestamp, project) pairs. Without this, active time, touchpoint counts,
and flow are all inflated. (The same duplication affects `examine`'s token
totals today; fixing it for attention should note the wider issue.)

### Known bug fixed alongside

`ClaudeCodeParser::parse_session` currently counts every `type: "user"`
event as a human turn, including tool results. This inflates human turn
counts and deflates AI:Human ratios everywhere they are used (signals,
reflections, authorship). The parser fix lands with this feature and the
commit message discloses the measurement correction. Existing tests that
encode the old counting change accordingly.

## Definitions

All thresholds are configurable (see Configuration); defaults in parentheses.

- **Touchpoint** — timestamp of a genuine human prompt in any session,
  after dedup. The analysis window (`--days N`) filters at touchpoint
  level, not session level: an old session's recent prompts are included.
- **Global timeline** — all touchpoints across all sessions/projects within
  the analysis window, sorted by time.
- **Active time** — for each consecutive global touchpoint pair with gap *g*:
  - *g* ≤ `idle_minutes` (15): count *g* as active time.
  - *g* > `idle_minutes`: count `engagement_floor_minutes` (2).
  - The final touchpoint of the window earns the engagement floor.
  Totals are exact; **per-project attribution is a range**: computed once
  attributing each gap to the earlier touchpoint's project (reviewing what
  you just prompted) and once to the later (preparing what you prompt
  next). Per-project active time is reported as the band between the two,
  because the heuristic's bias direction is unknowable per gap.
  Aggregated per project, per session, and per **local-timezone** day —
  all day bucketing uses the machine's local timezone, never UTC.
- **Context switch** — consecutive touchpoints on different projects with
  gap ≤ `idle_minutes`. Larger gaps are resumptions, not switches.
- **Dwell** — duration of a maximal run of consecutive same-project
  touchpoints (bounded by switches or idle gaps).
- **Flow episode** — maximal touchpoint run where every consecutive gap
  either is ≤ `flow_gap_minutes` (10) **or is waiting-on-AI**: an
  AI-working span belonging to a session the user touches next covers ≥
  half of the gap (capped at `idle_minutes` — beyond that you left).
  Total span must be ≥ `flow_min_minutes` (20). Without the
  waiting-on-AI rule, single-project agentic work — long agent runs, brief
  check-ins — would systematically read as "no flow", a false negative
  correlated with the very style being measured. Episodes are tagged
  `single_project` or `multi_project` with the projects involved.
- **AI-working span** — per session: from each human prompt to the last
  logged event of that session before the session's next human prompt (or
  the session's last event). Approximates when the assistant was working.
- **Orchestration** — wall-clock time during which ≥ 2 sessions had active
  AI-working spans, reported two ways: raw overlap, and overlap
  **intersected with the user's active attention windows** (excluding
  kicked-off-and-walked-away time). Also reports max simultaneous
  sessions. The intersected figure is the headline; raw is context.

## Architecture

### Parser and models (`src/ai_tools/`)

- `AiSession` gains `interactions: Vec<Interaction>` where
  `Interaction { human_at: DateTime<Utc>, ai_until: Option<DateTime<Utc>> }`
  (message UUID retained during parsing for cross-file dedup).
- `parse_session` collects interactions using the human-prompt
  discrimination above, and counts `human_turns` from genuine prompts only.
- `TurnCounts` gains `machine: u64` (tool results and other machine-authored
  user-type events) so the old and new counting are both visible.
- Sidechain and meta events are excluded from turns and interactions.

**Breaking measurement change:** corrected human-turn counting raises
AI:Human ratios substantially (tool results no longer count as human
turns). The default `ai_dependency_concern` threshold is recalibrated in
the same change, and the README notes that user-configured ratio
thresholds in existing `conscience.yaml` files mean something different
after upgrading.

### Analysis (`src/analysis/attention.rs`)

Pure functions over `&[AiSession]` plus thresholds:

```
pub struct AttentionAnalysis {
    window_days: u32,
    per_project: Vec<ProjectAttention>,   // active time as [earlier, later] range
    per_day: Vec<DayAttention>,           // local-tz day: active time, switches
    switches_per_day: f64,
    dwell: DwellStats,                    // median/mean/max dwell
    flow_episodes: Vec<FlowEpisode>,      // start, duration, projects, kind
    orchestration: OrchestrationStats,    // raw + attention-intersected overlap
    thresholds_used: AttentionThresholds,
}
```

All structs serialize for `--json` (and eventual dashboard reuse, out of
scope here).

### CLI (`src/main.rs`)

`conscience attention [--days 7] [--project <path>] [--json] [--html <path>]`

- Default window 7 days (attention patterns are more legible weekly than
  monthly).
- Terminal output: per-project table, switches/day, dwell stats, flow
  episode list, orchestration summary. Header carries the caveat that
  active time is a floor derived only from Claude Code activity.
- `--json` prints the full `AttentionAnalysis`.
- `--html <path>` additionally writes the timeline visualization and prints
  the path.

### Reflection integration (anti-Goodhart)

Metrics alone invite optimizing the number ("maximize flow minutes") — the
proxy-metric trap MH 159 warns against. Two integrations keep this a
discernment tool:

- The terminal output **ends with a reflection question** generated from
  the data (e.g. "Most of your flow this week spanned 3 projects — does
  that feel like richness or fragmentation? What would an ideal week look
  like?").
- `ethics::reflection::generate_reflections` gains an attention-aware
  builder (HumanAgency/DeveloperGrowth principle) when attention data is
  available, so `reflect` sessions can engage with it.

### HTML timeline (`src/analysis/attention_html.rs`)

One self-contained file: inline SVG + embedded CSS, no external assets, no
scripts required for the core view. Layout: one section per day; within it a
swimlane per project. Rendered elements:

- touchpoint tick marks,
- attention spans (project-colored),
- AI-working spans (lighter underlay),
- flow episodes (translucent band spanning lanes, labeled single/multi).

A small legend and the thresholds used are printed in the footer.

## Configuration

`conscience.yaml` gains an `attention` block under thresholds:

```yaml
thresholds:
  attention:
    idle_minutes: 15
    engagement_floor_minutes: 2
    flow_gap_minutes: 10
    flow_min_minutes: 20
    project_aliases:
      # worktrees, scratchpads, and subdirs folded into their logical project
      "/private/tmp/worktrees/conscience-*": conscience
```

Subsidiarity: each user tunes what "idle" and "flow" mean for their own
style. Defaults apply when absent; `thresholds_used` echoes them in output.
`project_aliases` maps path globs to a logical project name, because
worktree and scratchpad sessions otherwise appear as distinct projects and
pollute switch counts and lanes. (Git-root normalization is a possible
future improvement; the alias map is the v1 answer.)

## Honest-measurement caveats (shown to users)

- Active time counts only visible Claude Code interactions; reading,
  meetings, and non-AI coding are invisible. Totals are floors.
- Per-project attribution is a range precisely because the underlying
  heuristic is undecidable: between a prompt on X and a prompt on Y,
  attention may have been on either. The band's width is the honesty.
- Flow inference is a hypothesis generator for reflection, not a judgment.

## Privacy

Extracted data: timestamps, project paths, session ids only. No message
content, no file paths from conversations, nothing leaves the machine. The
HTML file is written where the user asks and shared only if they share it.

**Standing constraint for future work:** attention data is personal-scope.
Per-developer attention timelines are the most surveillance-ready data
conscience touches; any future dashboard aggregation requires explicit
per-user consent and team-level-only presentation. This constraint is part
of the design, not an afterthought for whoever builds the dashboard piece.

## Testing

- TDD throughout.
- `analysis/attention.rs`: synthetic timelines covering gap capping, floor
  attribution, both attribution directions (range bounds), switch vs
  resumption, dwell boundaries, flow episode boundaries (gap exactly at
  threshold, minimum-length cutoff), the waiting-on-AI flow rule,
  single/multi tagging, raw vs intersected overlap, touchpoint-level
  window filtering, and local-timezone day bucketing (a late-evening
  fixture that lands on different UTC/local days).
- Parser: fixture `.jsonl` files (first direct parser tests) covering human
  vs tool_result discrimination, sidechain/meta exclusion, interaction
  spans, machine-turn counting, and a duplicated-history fixture proving
  cross-file dedup.
- HTML: smoke tests (contains SVG lanes/ticks for a known input).

## Out of scope (future)

- Dashboard push of attention summaries (conscience-dashboard).
- Per-event assistant/tool timelines (only approximated spans now).
- Other AI tools' logs (Copilot etc.) — follows the parser roadmap.
- Cross-developer/team aggregation.
