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

### Known bug fixed alongside

`ClaudeCodeParser::parse_session` currently counts every `type: "user"`
event as a human turn, including tool results. This inflates human turn
counts and deflates AI:Human ratios everywhere they are used (signals,
reflections, authorship). The parser fix lands with this feature and the
commit message discloses the measurement correction. Existing tests that
encode the old counting change accordingly.

## Definitions

All thresholds are configurable (see Configuration); defaults in parentheses.

- **Touchpoint** — timestamp of a genuine human prompt in any session.
- **Global timeline** — all touchpoints across all sessions/projects within
  the analysis window, sorted by time.
- **Active time** — for each consecutive global touchpoint pair with gap *g*:
  - *g* ≤ `idle_minutes` (15): count *g* as active time, attributed to the
    **earlier** touchpoint's project.
  - *g* > `idle_minutes`: count `engagement_floor_minutes` (2) for the
    earlier touchpoint.
  - The final touchpoint of the window earns the engagement floor.
  Aggregated per project, per session, per day.
- **Context switch** — consecutive touchpoints on different projects with
  gap ≤ `idle_minutes`. Larger gaps are resumptions, not switches.
- **Dwell** — duration of a maximal run of consecutive same-project
  touchpoints (bounded by switches or idle gaps).
- **Flow episode** — maximal touchpoint run where every consecutive gap ≤
  `flow_gap_minutes` (10) and total span ≥ `flow_min_minutes` (20). Tagged
  `single_project` or `multi_project` with the list of projects involved.
- **AI-working span** — per session: from each human prompt to the last
  logged event of that session before the session's next human prompt (or
  the session's last event). Approximates when the assistant was working.
- **Orchestration** — wall-clock time during which ≥ 2 sessions had active
  AI-working spans; reported as total overlap time, ratio of overlap time to
  total active AI time, and max simultaneous sessions.

## Architecture

### Parser and models (`src/ai_tools/`)

- `AiSession` gains `interactions: Vec<Interaction>` where
  `Interaction { human_at: DateTime<Utc>, ai_until: Option<DateTime<Utc>> }`.
- `parse_session` collects interactions using the human-prompt
  discrimination above, and counts `human_turns` from genuine prompts only.
- Sidechain and meta events are excluded from turns and interactions.

### Analysis (`src/analysis/attention.rs`)

Pure functions over `&[AiSession]` plus thresholds:

```
pub struct AttentionAnalysis {
    window_days: u32,
    per_project: Vec<ProjectAttention>,   // active time, touchpoints, sessions
    per_day: Vec<DayAttention>,           // active time, switches, projects
    switches_per_day: f64,
    dwell: DwellStats,                    // median/mean/max dwell
    flow_episodes: Vec<FlowEpisode>,      // start, duration, gaps, projects, kind
    orchestration: OrchestrationStats,    // overlap time, ratio, max concurrent
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
```

Subsidiarity: each user tunes what "idle" and "flow" mean for their own
style. Defaults apply when absent; `thresholds_used` echoes them in output.

## Honest-measurement caveats (shown to users)

- Active time counts only visible Claude Code interactions; reading,
  meetings, and non-AI coding are invisible. Totals are floors.
- Gap attribution (earlier project) is a heuristic; between a prompt on X
  and a prompt on Y, attention may have been on either.
- Flow inference is a hypothesis generator for reflection, not a judgment.

## Privacy

Extracted data: timestamps, project paths, session ids only. No message
content, no file paths from conversations, nothing leaves the machine. The
HTML file is written where the user asks and shared only if they share it.

## Testing

- TDD throughout.
- `analysis/attention.rs`: synthetic timelines covering gap capping, floor
  attribution, switch vs resumption, dwell boundaries, flow episode
  boundaries (gap exactly at threshold, minimum-length cutoff),
  single/multi tagging, overlap computation.
- Parser: fixture `.jsonl` files (first direct parser tests) covering human
  vs tool_result discrimination, sidechain exclusion, interaction spans.
- HTML: smoke tests (contains SVG lanes/ticks for a known input).

## Out of scope (future)

- Dashboard push of attention summaries (conscience-dashboard).
- Per-event assistant/tool timelines (only approximated spans now).
- Other AI tools' logs (Copilot etc.) — follows the parser roadmap.
- Cross-developer/team aggregation.
