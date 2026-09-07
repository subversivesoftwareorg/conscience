# Attention & Flow Analysis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `conscience attention` command that analyzes how a developer's attention moves across projects — active time (as honest ranges), context switches, flow episodes that may span projects, and concurrent-AI orchestration — from Claude Code logs, locally, with terminal stats and an optional self-contained HTML timeline.

**Architecture:** The Claude Code parser gains per-interaction timestamps (genuine human prompts only, with cross-file dedup keys) and a corrected human/machine turn split. A pure `analysis/attention` module computes all metrics from `&[AiSession]` plus configurable thresholds. A new CLI command renders terminal stats, JSON, and an inline-SVG HTML timeline. The output closes with a data-fed reflection question.

**Tech Stack:** Rust; existing deps only (clap, chrono, serde, comfy-table). No new crates.

**Spec:** `docs/superpowers/specs/2026-09-07-attention-analysis-design.md`

## Global Constraints

- Extracted data: timestamps, project paths, session ids only — never message content.
- All day bucketing uses a `chrono::FixedOffset` passed in (production: local offset), never UTC directly.
- Touchpoints MUST be deduplicated across session files (uuid, fallback timestamp+project).
- Thresholds defaults: `idle_minutes: 15`, `engagement_floor_minutes: 2`, `flow_gap_minutes: 10`, `flow_min_minutes: 20`.
- Per-project active time is a `[min, max]` range (earlier- vs later-attribution), never a single number.
- TDD: every task writes its failing test first and runs it before implementing.
- Repo is not rustfmt-clean; keep NEW files rustfmt-clean, do not reformat existing code.
- Commit after each task; commit messages disclose the turn-count measurement correction where relevant.

---

### Task 1: Corrected turn counting (human vs machine) in the parser

**Files:**
- Modify: `src/ai_tools/models.rs` (TurnCounts)
- Modify: `src/ai_tools/claude_code.rs` (parse_session)
- Create: `tests/fixtures/basic_session.jsonl`
- Create: `tests/parser_test.rs`
- Modify: `tests/signals_test.rs`, `tests/authorship_test.rs` (struct literals gain a field)

**Interfaces:**
- Produces: `TurnCounts { human, assistant, machine, total }` — `machine` counts tool_result/meta user-type events; `human` counts ONLY genuine prompts (string content, not sidechain, not meta, not starting with `"Caveat:"` or `"<local-command-stdout>"`). `total` stays `human + assistant`.

- [ ] **Step 1: Write the fixture.** Create `tests/fixtures/basic_session.jsonl` (real log shape; one genuine prompt, one tool_result, one sidechain prompt, one caveat message):

```jsonl
{"type":"user","uuid":"u1","timestamp":"2026-09-01T16:40:34.873Z","isSidechain":false,"cwd":"/home/dev/projA","message":{"role":"user","content":"please fix the bug"}}
{"type":"assistant","uuid":"a1","timestamp":"2026-09-01T16:40:43.409Z","isSidechain":false,"message":{"model":"claude-fable-5","usage":{"input_tokens":10,"output_tokens":20},"content":[{"type":"text","text":"ok"}]}}
{"type":"assistant","uuid":"a2","timestamp":"2026-09-01T16:40:45.055Z","isSidechain":false,"message":{"usage":{"input_tokens":1,"output_tokens":2},"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/tmp/x.rs"}}]}}
{"type":"user","uuid":"u2","timestamp":"2026-09-01T16:40:45.276Z","isSidechain":false,"message":{"role":"user","content":[{"type":"tool_result","content":"file contents"}]}}
{"type":"user","uuid":"u3","timestamp":"2026-09-01T16:40:50.000Z","isSidechain":true,"message":{"role":"user","content":"sidechain prompt"}}
{"type":"user","uuid":"u4","timestamp":"2026-09-01T16:40:55.000Z","isSidechain":false,"isMeta":true,"message":{"role":"user","content":"Caveat: the messages below were generated while running local commands"}}
{"type":"user","uuid":"u5","timestamp":"2026-09-01T16:41:05.654Z","isSidechain":false,"message":{"role":"user","content":"thanks, now add a test"}}
{"type":"assistant","uuid":"a3","timestamp":"2026-09-01T16:41:20.000Z","isSidechain":false,"message":{"usage":{"input_tokens":5,"output_tokens":9},"content":[{"type":"text","text":"done"}]}}
```

- [ ] **Step 2: Write the failing test.** Create `tests/parser_test.rs`:

```rust
use conscience::ai_tools::claude_code::ClaudeCodeParser;
use std::path::Path;

fn parse_fixture(name: &str) -> conscience::ai_tools::models::AiSession {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    ClaudeCodeParser::new()
        .parse_session_file("test-session", &path)
        .expect("fixture parses")
}

#[test]
fn human_turns_count_only_genuine_prompts() {
    let session = parse_fixture("basic_session.jsonl");
    assert_eq!(session.turns.human, 2, "u1 and u5 only");
    assert_eq!(session.turns.machine, 2, "tool_result u2 and caveat u4");
    assert_eq!(session.turns.assistant, 3);
    assert_eq!(session.turns.total, 5, "human + assistant");
}
```

This requires exposing the private `parse_session` as a public method: add `pub fn parse_session_file(&self, session_id: &str, path: &Path) -> Result<AiSession>` that delegates to the existing private fn.

- [ ] **Step 3: Run test, verify it fails.** `cargo test --test parser_test` — expected: compile error (`parse_session_file` not found), then after adding the delegate, assertion failure `human = 4` (old counting).

- [ ] **Step 4: Implement.** In `src/ai_tools/models.rs` add to `TurnCounts`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnCounts {
    pub human: u64,
    pub assistant: u64,
    #[serde(default)]
    pub machine: u64,
    pub total: u64,
}
```

In `claude_code.rs`, add the public delegate and a discrimination helper, and use it in the `"user"` match arm:

```rust
pub fn parse_session_file(&self, session_id: &str, path: &Path) -> Result<AiSession> {
    self.parse_session(session_id, path)
}

/// A genuine human prompt: string content, not sidechain, not meta,
/// not machine-injected caveat/local-command output.
fn is_human_prompt(value: &Value) -> bool {
    if value.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false) {
        return false;
    }
    if value.get("isMeta").and_then(|v| v.as_bool()).unwrap_or(false) {
        return false;
    }
    match value.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
        Some(text) => {
            !text.starts_with("Caveat:") && !text.starts_with("<local-command-stdout>")
        }
        None => false,
    }
}
```

In the `"user"` arm: `if is_human_prompt(&value) { human_turns += 1; } else { machine_turns += 1; }` (declare `let mut machine_turns = 0u64;`). Sidechain assistant events must not count either: at the top of the `"assistant"` arm, `if value.get("isSidechain").and_then(|v| v.as_bool()).unwrap_or(false) { continue; }` — but keep the timestamp scan above the match so started/ended stay whole-file. Set `machine: machine_turns` and `total: human_turns + assistant_turns` in the returned `TurnCounts`.

- [ ] **Step 5: Fix compiling tests.** `tests/signals_test.rs:78` and `tests/authorship_test.rs` build `TurnCounts` literals — add `machine: 0,` to each (grep for `TurnCounts {`).

- [ ] **Step 6: Run the full suite.** `cargo test` — all pass. If a signals/authorship test asserted on old human counts derived from fixtures (they build sessions directly, so counts are explicit — they should pass unchanged).

- [ ] **Step 7: Commit.**

```bash
git add -A && git commit -m "Fix human turn counting: tool results are machine turns

Measurement correction: type=user events with tool_result/meta content
no longer count as human turns. AI:Human ratios rise accordingly."
```

---

### Task 2: Interaction extraction (human_at, ai_until, uuid)

**Files:**
- Modify: `src/ai_tools/models.rs` (Interaction struct, AiSession field)
- Modify: `src/ai_tools/claude_code.rs` (parse_session)
- Test: `tests/parser_test.rs` (extend)

**Interfaces:**
- Produces: `Interaction { human_at: DateTime<Utc>, ai_until: Option<DateTime<Utc>>, uuid: Option<String> }`; `AiSession.interactions: Vec<Interaction>` (serde default). `ai_until` = timestamp of the session's last event before the next genuine prompt (or the session's last event).

- [ ] **Step 1: Write the failing test** (append to `tests/parser_test.rs`):

```rust
#[test]
fn interactions_capture_prompt_and_response_end() {
    let session = parse_fixture("basic_session.jsonl");
    assert_eq!(session.interactions.len(), 2);

    let first = &session.interactions[0];
    assert_eq!(first.uuid.as_deref(), Some("u1"));
    assert_eq!(first.human_at.to_rfc3339(), "2026-09-01T16:40:34.873+00:00");
    // last event before u5 is u4 (16:40:55)
    assert_eq!(
        first.ai_until.unwrap().to_rfc3339(),
        "2026-09-01T16:40:55+00:00"
    );

    let second = &session.interactions[1];
    assert_eq!(second.uuid.as_deref(), Some("u2".replace("u2", "u5")).as_deref());
    // last event of the file is a3 (16:41:20)
    assert_eq!(
        second.ai_until.unwrap().to_rfc3339(),
        "2026-09-01T16:41:20+00:00"
    );
}
```

(Write the second uuid assertion plainly as `Some("u5")` — shown here expanded to make the intent unmistakable.)

- [ ] **Step 2: Run, verify failure.** `cargo test --test parser_test` — compile error: no `interactions` field.

- [ ] **Step 3: Implement.** In `models.rs`:

```rust
/// One human prompt and the approximate span the AI worked on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub human_at: DateTime<Utc>,
    pub ai_until: Option<DateTime<Utc>>,
    pub uuid: Option<String>,
}
```

Add `#[serde(default)] pub interactions: Vec<Interaction>` to `AiSession`. In `parse_session`, track:

```rust
let mut interactions: Vec<Interaction> = Vec::new();
let mut last_event_ts: Option<DateTime<Utc>> = None;
```

Inside the timestamp scan (every event with a parseable timestamp): before updating, if this event is a genuine human prompt, close the open interaction first: `if let Some(open) = interactions.last_mut() { if open.ai_until.is_none() { open.ai_until = last_event_ts; } }`, then push `Interaction { human_at: dt, ai_until: None, uuid: value.get("uuid").and_then(|v| v.as_str()).map(String::from) }`. Then always `last_event_ts = Some(dt)`. After the loop, close the final open interaction the same way. (Call `is_human_prompt` only for `type == "user"` events.) Set `interactions` on the returned session; other constructors of `AiSession` in tests get `interactions: Vec::new()` (compile errors point at them — `tests/signals_test.rs` and `tests/authorship_test.rs` helpers).

- [ ] **Step 4: Run tests.** `cargo test` — all pass.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Extract per-interaction timestamps from Claude Code sessions"`

---

### Task 3: Attention thresholds in the manifest + ratio recalibration

**Files:**
- Modify: `src/ethics/manifest.rs`
- Modify: `tests/manifest_test.rs`

**Interfaces:**
- Produces: `AttentionThresholds { idle_minutes: f64, engagement_floor_minutes: f64, flow_gap_minutes: f64, flow_min_minutes: f64, project_aliases: BTreeMap<String, String> }` with `Default`; `Thresholds.attention: AttentionThresholds` (serde default). New ratio defaults: `ai_dependency_info = 6.0`, `ai_dependency_concern = 12.0`.

- [ ] **Step 1: Write the failing tests** (append to `tests/manifest_test.rs`):

```rust
#[test]
fn attention_thresholds_default_and_parse() {
    let manifest: Manifest = serde_yaml::from_str("{}").unwrap();
    let a = &manifest.thresholds.attention;
    assert_eq!(a.idle_minutes, 15.0);
    assert_eq!(a.engagement_floor_minutes, 2.0);
    assert_eq!(a.flow_gap_minutes, 10.0);
    assert_eq!(a.flow_min_minutes, 20.0);
    assert!(a.project_aliases.is_empty());

    let yaml = r#"
thresholds:
  attention:
    idle_minutes: 20
    project_aliases:
      "/tmp/worktrees/*": conscience
"#;
    let m: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(m.thresholds.attention.idle_minutes, 20.0);
    assert_eq!(m.thresholds.attention.flow_gap_minutes, 10.0, "unset keeps default");
    assert_eq!(
        m.thresholds.attention.project_aliases.get("/tmp/worktrees/*").map(String::as_str),
        Some("conscience")
    );
}

#[test]
fn ai_dependency_defaults_recalibrated_for_corrected_counting() {
    let manifest: Manifest = serde_yaml::from_str("{}").unwrap();
    assert_eq!(manifest.thresholds.ai_dependency_info, 6.0);
    assert_eq!(manifest.thresholds.ai_dependency_concern, 12.0);
}
```

- [ ] **Step 2: Run, verify failure.** `cargo test --test manifest_test` — compile error (no `attention` field).

- [ ] **Step 3: Implement** in `manifest.rs` (near `Thresholds`):

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionThresholds {
    #[serde(default = "default_idle_minutes")]
    pub idle_minutes: f64,
    #[serde(default = "default_engagement_floor_minutes")]
    pub engagement_floor_minutes: f64,
    #[serde(default = "default_flow_gap_minutes")]
    pub flow_gap_minutes: f64,
    #[serde(default = "default_flow_min_minutes")]
    pub flow_min_minutes: f64,
    #[serde(default)]
    pub project_aliases: BTreeMap<String, String>,
}

impl Default for AttentionThresholds {
    fn default() -> Self {
        Self {
            idle_minutes: default_idle_minutes(),
            engagement_floor_minutes: default_engagement_floor_minutes(),
            flow_gap_minutes: default_flow_gap_minutes(),
            flow_min_minutes: default_flow_min_minutes(),
            project_aliases: BTreeMap::new(),
        }
    }
}

fn default_idle_minutes() -> f64 { 15.0 }
fn default_engagement_floor_minutes() -> f64 { 2.0 }
fn default_flow_gap_minutes() -> f64 { 10.0 }
fn default_flow_min_minutes() -> f64 { 20.0 }
```

Add `#[serde(default)] pub attention: AttentionThresholds` to `Thresholds` (and `attention: AttentionThresholds::default()` in its `Default` impl). Change `default_ai_dependency_info()` to `6.0` and `default_ai_dependency_concern()` to `12.0`.

- [ ] **Step 4: Fix the existing default assertions.** `tests/manifest_test.rs::test_defaults_applied` asserts `ai_dependency_concern == 3.0` — update to `12.0` (and info to `6.0` if asserted).

- [ ] **Step 5: Run tests.** `cargo test` — all pass.

- [ ] **Step 6: Commit.** `git add -A && git commit -m "Add attention thresholds; recalibrate AI dependency defaults for corrected counting"`

---

### Task 4: Touchpoint collection — dedup, aliases, windowing

**Files:**
- Create: `src/analysis/attention.rs`
- Modify: `src/analysis/mod.rs` (add `pub mod attention;`)
- Create: `tests/attention_test.rs`

**Interfaces:**
- Consumes: `AiSession.interactions`, `AttentionThresholds.project_aliases`.
- Produces:
  ```rust
  pub struct Touchpoint {
      pub at: DateTime<Utc>,
      pub project: String,
      pub session_id: String,
      pub ai_until: Option<DateTime<Utc>>,
  }
  pub fn collect_touchpoints(
      sessions: &[AiSession],
      th: &AttentionThresholds,
      since: DateTime<Utc>,
  ) -> Vec<Touchpoint>  // deduped, alias-applied, window-filtered, sorted by at
  pub fn apply_alias(path: &str, aliases: &BTreeMap<String, String>) -> String
  ```
  Alias rule: pattern ending in `*` is a prefix match on everything before the `*`; otherwise exact match; first match in BTreeMap order wins; no match returns the path unchanged.

- [ ] **Step 1: Write the failing tests.** Create `tests/attention_test.rs`:

```rust
use chrono::{DateTime, Duration, TimeZone, Utc};
use conscience::ai_tools::models::*;
use conscience::analysis::attention::*;
use conscience::ethics::manifest::AttentionThresholds;
use std::collections::BTreeMap;

fn base() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap()
}

fn session(id: &str, project: &str, prompts: &[(i64, Option<i64>)]) -> AiSession {
    // prompts: (minutes_after_base, ai_until_minutes_after_base)
    AiSession {
        tool: AiTool::ClaudeCode,
        session_id: id.to_string(),
        project_path: Some(project.to_string()),
        started_at: Some(base()),
        ended_at: None,
        model: None,
        work_categories: vec![WorkCategory::Code],
        turns: TurnCounts::default(),
        tokens: TokenUsage::default(),
        tools_used: Default::default(),
        files_touched: Vec::new(),
        bash_commands: Vec::new(),
        agent_actions: Vec::new(),
        git_branch: None,
        interactions: prompts
            .iter()
            .map(|(m, until)| Interaction {
                human_at: base() + Duration::minutes(*m),
                ai_until: until.map(|u| base() + Duration::minutes(u)),
                uuid: Some(format!("{}-{}", id, m)),
            })
            .collect(),
    }
}

fn th() -> AttentionThresholds {
    AttentionThresholds::default()
}

fn long_ago() -> DateTime<Utc> {
    base() - Duration::days(30)
}

#[test]
fn touchpoints_dedupe_by_uuid_across_files() {
    let s1 = session("orig", "/p/a", &[(0, Some(1)), (5, Some(6))]);
    let mut s2 = session("resumed", "/p/a", &[(0, Some(1)), (5, Some(6)), (10, None)]);
    // resumed file copied history: same uuids for the first two
    s2.interactions[0].uuid = s1.interactions[0].uuid.clone();
    s2.interactions[1].uuid = s1.interactions[1].uuid.clone();

    let tps = collect_touchpoints(&[s1, s2], &th(), long_ago());
    assert_eq!(tps.len(), 3, "copied history deduped");
}

#[test]
fn touchpoints_dedupe_without_uuid_by_time_and_project() {
    let mut s1 = session("a", "/p/a", &[(0, None)]);
    let mut s2 = session("b", "/p/a", &[(0, None)]);
    s1.interactions[0].uuid = None;
    s2.interactions[0].uuid = None;

    let tps = collect_touchpoints(&[s1, s2], &th(), long_ago());
    assert_eq!(tps.len(), 1);
}

#[test]
fn touchpoints_filtered_by_window_not_session() {
    let s = session("a", "/p/a", &[(-60 * 24 * 10, None), (0, None)]);
    let since = base() - Duration::days(7);
    let tps = collect_touchpoints(&[s], &th(), since);
    assert_eq!(tps.len(), 1, "old prompt excluded, recent kept");
}

#[test]
fn project_aliases_fold_worktrees() {
    let mut t = th();
    t.project_aliases
        .insert("/tmp/worktrees/*".to_string(), "conscience".to_string());
    let s1 = session("a", "/tmp/worktrees/conscience-x", &[(0, None)]);
    let s2 = session("b", "/home/dev/conscience", &[(5, None)]);

    let tps = collect_touchpoints(&[s1, s2], &t, long_ago());
    assert_eq!(tps[0].project, "conscience");
    assert_eq!(tps[1].project, "/home/dev/conscience");
}
```

- [ ] **Step 2: Run, verify failure.** `cargo test --test attention_test` — compile error: module `attention` not found.

- [ ] **Step 3: Implement.** `src/analysis/mod.rs`: add `pub mod attention;`. Create `src/analysis/attention.rs`:

```rust
use crate::ai_tools::models::AiSession;
use crate::ethics::manifest::AttentionThresholds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// A moment of genuine human attention: one deduplicated prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Touchpoint {
    pub at: DateTime<Utc>,
    pub project: String,
    pub session_id: String,
    pub ai_until: Option<DateTime<Utc>>,
}

pub fn apply_alias(path: &str, aliases: &BTreeMap<String, String>) -> String {
    for (pattern, name) in aliases {
        if let Some(prefix) = pattern.strip_suffix('*') {
            if path.starts_with(prefix) {
                return name.clone();
            }
        } else if path == pattern {
            return name.clone();
        }
    }
    path.to_string()
}

pub fn collect_touchpoints(
    sessions: &[AiSession],
    th: &AttentionThresholds,
    since: DateTime<Utc>,
) -> Vec<Touchpoint> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut tps = Vec::new();

    for s in sessions {
        let project = apply_alias(s.project_path.as_deref().unwrap_or("(unknown)"), &th.project_aliases);
        for i in &s.interactions {
            if i.human_at < since {
                continue;
            }
            let key = match &i.uuid {
                Some(u) => format!("uuid:{}", u),
                None => format!("ts:{}:{}", i.human_at.timestamp_millis(), project),
            };
            if !seen.insert(key) {
                continue;
            }
            tps.push(Touchpoint {
                at: i.human_at,
                project: project.clone(),
                session_id: s.session_id.clone(),
                ai_until: i.ai_until,
            });
        }
    }

    tps.sort_by_key(|t| t.at);
    tps
}
```

- [ ] **Step 4: Run tests.** `cargo test --test attention_test` — pass; then `cargo test` for the suite.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Collect deduplicated attention touchpoints with project aliasing"`

---

### Task 5: Active time — ranges and local-day bucketing

**Files:**
- Modify: `src/analysis/attention.rs`
- Test: `tests/attention_test.rs` (extend)

**Interfaces:**
- Produces:
  ```rust
  pub struct ProjectAttention {
      pub project: String,
      pub touchpoints: u64,
      pub sessions: u64,
      pub active_minutes_min: f64,
      pub active_minutes_max: f64,
  }
  pub struct DayAttention {
      pub date: chrono::NaiveDate, // local date per tz offset
      pub active_minutes: f64,
      pub switches: u64,           // filled by Task 6; 0 here
      pub projects: Vec<String>,
  }
  pub struct ActiveTime {
      pub total_minutes: f64,
      pub per_project: Vec<ProjectAttention>,
      pub per_day: Vec<DayAttention>,
  }
  pub fn compute_active_time(
      tps: &[Touchpoint],
      th: &AttentionThresholds,
      tz: chrono::FixedOffset,
  ) -> ActiveTime
  ```
  Semantics: gap ≤ idle → full gap active; gap > idle (and final touchpoint) → engagement floor. Total is attribution-independent. Per-project min/max = the smaller/larger of earlier-attribution and later-attribution sums. Day attribution: the earlier touchpoint's local date.

- [ ] **Step 1: Write the failing tests** (append):

```rust
use chrono::FixedOffset;

fn utc_tz() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

#[test]
fn active_time_caps_gaps_and_floors_idle() {
    // A@0, A@10 (gap 10 ≤ 15: counts 10), A@40 (gap 30 > 15: floor 2), final: floor 2
    let s = session("a", "/p/a", &[(0, None), (10, None), (40, None)]);
    let tps = collect_touchpoints(&[s], &th(), long_ago());
    let at = compute_active_time(&tps, &th(), utc_tz());
    assert_eq!(at.total_minutes, 10.0 + 2.0 + 2.0);
}

#[test]
fn per_project_active_time_is_a_range() {
    // A@0 -> B@10: the 10-min gap is A's (earlier) or B's (later)
    let a = session("a", "/p/a", &[(0, None)]);
    let b = session("b", "/p/b", &[(10, None)]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let at = compute_active_time(&tps, &th(), utc_tz());

    let pa = at.per_project.iter().find(|p| p.project == "/p/a").unwrap();
    let pb = at.per_project.iter().find(|p| p.project == "/p/b").unwrap();
    // earlier-attribution: A gets 10 + B floor 2; later-attribution: A floor... A has no
    // incoming gap so A gets 0 + ... — assert the bounds we defined:
    assert_eq!(pa.active_minutes_max, 10.0);
    assert!(pa.active_minutes_min < pa.active_minutes_max);
    assert_eq!(pb.active_minutes_max, 10.0 + 2.0);
    assert_eq!(pb.active_minutes_min, 2.0);
}

#[test]
fn day_bucketing_uses_local_timezone() {
    // 2026-09-02 03:00 UTC == 2026-09-01 22:00 in UTC-5
    let s = session("a", "/p/a", &[(15 * 60, None)]); // base 12:00 + 15h = 03:00 next day UTC
    let tps = collect_touchpoints(&[s], &th(), long_ago());

    let central = FixedOffset::west_opt(5 * 3600).unwrap();
    let at = compute_active_time(&tps, &th(), central);
    assert_eq!(at.per_day.len(), 1);
    assert_eq!(at.per_day[0].date.to_string(), "2026-09-01");

    let at_utc = compute_active_time(&tps, &th(), utc_tz());
    assert_eq!(at_utc.per_day[0].date.to_string(), "2026-09-02");
}
```

Note on the range test: with earlier-attribution A earns the 10-min gap and B earns only its final floor (2); with later-attribution B earns the gap and its floor (12) while A earns 0. So `pa = [0, 10]`, `pb = [2, 12]`.  The assertions encode exactly that.

- [ ] **Step 2: Run, verify failure.** Compile error: `compute_active_time` not found.

- [ ] **Step 3: Implement** in `attention.rs`:

```rust
use chrono::FixedOffset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAttention {
    pub project: String,
    pub touchpoints: u64,
    pub sessions: u64,
    pub active_minutes_min: f64,
    pub active_minutes_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayAttention {
    pub date: chrono::NaiveDate,
    pub active_minutes: f64,
    pub switches: u64,
    pub projects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTime {
    pub total_minutes: f64,
    pub per_project: Vec<ProjectAttention>,
    pub per_day: Vec<DayAttention>,
}

fn minutes_between(a: DateTime<Utc>, b: DateTime<Utc>) -> f64 {
    (b - a).num_seconds() as f64 / 60.0
}

pub fn compute_active_time(
    tps: &[Touchpoint],
    th: &AttentionThresholds,
    tz: FixedOffset,
) -> ActiveTime {
    let mut earlier: BTreeMap<String, f64> = BTreeMap::new();
    let mut later: BTreeMap<String, f64> = BTreeMap::new();
    let mut per_day: BTreeMap<chrono::NaiveDate, (f64, Vec<String>)> = BTreeMap::new();
    let mut total = 0.0;

    for (i, tp) in tps.iter().enumerate() {
        let credit = match tps.get(i + 1) {
            Some(next) => {
                let gap = minutes_between(tp.at, next.at);
                if gap <= th.idle_minutes {
                    gap
                } else {
                    th.engagement_floor_minutes
                }
            }
            None => th.engagement_floor_minutes,
        };
        total += credit;

        // earlier-attribution: this touchpoint's project owns the credit
        *earlier.entry(tp.project.clone()).or_insert(0.0) += credit;
        // later-attribution: the NEXT touchpoint's project owns real gaps;
        // floors stay with their own touchpoint
        let later_owner = match tps.get(i + 1) {
            Some(next) if minutes_between(tp.at, next.at) <= th.idle_minutes => {
                next.project.clone()
            }
            _ => tp.project.clone(),
        };
        *later.entry(later_owner).or_insert(0.0) += credit;

        let day = tp.at.with_timezone(&tz).date_naive();
        let entry = per_day.entry(day).or_insert((0.0, Vec::new()));
        entry.0 += credit;
        if !entry.1.contains(&tp.project) {
            entry.1.push(tp.project.clone());
        }
    }

    let mut counts: BTreeMap<String, (u64, HashSet<String>)> = BTreeMap::new();
    for tp in tps {
        let e = counts.entry(tp.project.clone()).or_default();
        e.0 += 1;
        e.1.insert(tp.session_id.clone());
    }

    let per_project = counts
        .into_iter()
        .map(|(project, (n, sess))| {
            let a = *earlier.get(&project).unwrap_or(&0.0);
            let b = *later.get(&project).unwrap_or(&0.0);
            ProjectAttention {
                project,
                touchpoints: n,
                sessions: sess.len() as u64,
                active_minutes_min: a.min(b),
                active_minutes_max: a.max(b),
            }
        })
        .collect();

    let per_day = per_day
        .into_iter()
        .map(|(date, (active_minutes, projects))| DayAttention {
            date,
            active_minutes,
            switches: 0,
            projects,
        })
        .collect();

    ActiveTime { total_minutes: total, per_project, per_day }
}
```

- [ ] **Step 4: Run tests.** `cargo test --test attention_test` then `cargo test`.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Compute active time with attribution ranges and local-day bucketing"`

---

### Task 6: Context switches and dwell

**Files:**
- Modify: `src/analysis/attention.rs`
- Test: `tests/attention_test.rs` (extend)

**Interfaces:**
- Produces:
  ```rust
  pub struct DwellStats { pub median_minutes: f64, pub mean_minutes: f64, pub max_minutes: f64 }
  pub fn compute_switches_dwell(tps: &[Touchpoint], th: &AttentionThresholds) -> (u64, DwellStats)
  ```
  Switch: consecutive touchpoints, different project, gap ≤ idle. Dwell run: maximal same-project run not broken by an idle gap; its duration = last.at − first.at (single-touchpoint run = 0, still included).

- [ ] **Step 1: Write the failing tests** (append):

```rust
#[test]
fn switches_ignore_idle_resumptions() {
    // A@0 -> B@5 (switch), B@10 -> A@60 (gap 50 > idle: resumption, not switch)
    let a = session("a", "/p/a", &[(0, None), (60, None)]);
    let b = session("b", "/p/b", &[(5, None), (10, None)]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let (switches, dwell) = compute_switches_dwell(&tps, &th());
    assert_eq!(switches, 1);
    // dwell runs: [A@0] (0 min), [B@5,B@10] (5 min), [A@60] (0 min)
    assert_eq!(dwell.max_minutes, 5.0);
}
```

- [ ] **Step 2: Run, verify failure.** `compute_switches_dwell` not found.

- [ ] **Step 3: Implement:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DwellStats {
    pub median_minutes: f64,
    pub mean_minutes: f64,
    pub max_minutes: f64,
}

pub fn compute_switches_dwell(tps: &[Touchpoint], th: &AttentionThresholds) -> (u64, DwellStats) {
    let mut switches = 0u64;
    let mut runs: Vec<f64> = Vec::new();
    let mut run_start: Option<usize> = None;

    for i in 0..tps.len() {
        if run_start.is_none() {
            run_start = Some(i);
        }
        let boundary = match tps.get(i + 1) {
            None => true,
            Some(next) => {
                let gap = minutes_between(tps[i].at, next.at);
                let idle = gap > th.idle_minutes;
                let switched = next.project != tps[i].project && !idle;
                if switched {
                    switches += 1;
                }
                idle || switched
            }
        };
        if boundary {
            let start = run_start.take().unwrap();
            runs.push(minutes_between(tps[start].at, tps[i].at));
        }
    }

    let stats = if runs.is_empty() {
        DwellStats { median_minutes: 0.0, mean_minutes: 0.0, max_minutes: 0.0 }
    } else {
        let mut sorted = runs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        DwellStats {
            median_minutes: sorted[sorted.len() / 2],
            mean_minutes: runs.iter().sum::<f64>() / runs.len() as f64,
            max_minutes: sorted[sorted.len() - 1],
        }
    };

    (switches, stats)
}
```

- [ ] **Step 4: Run tests.** `cargo test` — pass.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Compute context switches and dwell statistics"`

---

### Task 7: Flow episodes with the waiting-on-AI rule

**Files:**
- Modify: `src/analysis/attention.rs`
- Test: `tests/attention_test.rs` (extend)

**Interfaces:**
- Produces:
  ```rust
  pub struct FlowEpisode {
      pub start: DateTime<Utc>,
      pub end: DateTime<Utc>,
      pub minutes: f64,
      pub touchpoints: u64,
      pub projects: Vec<String>,
      pub multi_project: bool,
  }
  pub fn compute_flow_episodes(tps: &[Touchpoint], th: &AttentionThresholds) -> Vec<FlowEpisode>
  ```
  A gap continues an episode if gap ≤ flow_gap, OR (gap ≤ idle AND an earlier touchpoint in the *next* touchpoint's session has `ai_until` covering ≥ half the gap). Episode kept if span ≥ flow_min.

- [ ] **Step 1: Write the failing tests** (append):

```rust
#[test]
fn flow_episode_spans_projects() {
    // Prompts every 5 min alternating projects for 30 min: one multi-project episode
    let a = session("a", "/p/a", &[(0, None), (10, None), (20, None), (30, None)]);
    let b = session("b", "/p/b", &[(5, None), (15, None), (25, None)]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let eps = compute_flow_episodes(&tps, &th());
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].minutes, 30.0);
    assert!(eps[0].multi_project);
    assert_eq!(eps[0].touchpoints, 7);
}

#[test]
fn waiting_on_ai_does_not_break_single_project_flow() {
    // One project; agent runs 12 min between prompts (gap > flow_gap of 10),
    // but the session's AI span covers the whole gap -> flow continues.
    let s = session("a", "/p/a", &[(0, Some(12)), (12, Some(24)), (24, None)]);
    let tps = collect_touchpoints(&[s], &th(), long_ago());
    let eps = compute_flow_episodes(&tps, &th());
    assert_eq!(eps.len(), 1, "waiting on AI is not a flow break");
    assert_eq!(eps[0].minutes, 24.0);
    assert!(!eps[0].multi_project);
}

#[test]
fn short_bursts_are_not_flow() {
    let s = session("a", "/p/a", &[(0, None), (5, None)]); // span 5 < flow_min 20
    let tps = collect_touchpoints(&[s], &th(), long_ago());
    assert!(compute_flow_episodes(&tps, &th()).is_empty());
}
```

- [ ] **Step 2: Run, verify failure.** `compute_flow_episodes` not found.

- [ ] **Step 3: Implement:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEpisode {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub minutes: f64,
    pub touchpoints: u64,
    pub projects: Vec<String>,
    pub multi_project: bool,
}

pub fn compute_flow_episodes(tps: &[Touchpoint], th: &AttentionThresholds) -> Vec<FlowEpisode> {
    if tps.is_empty() {
        return Vec::new();
    }

    let mut episodes = Vec::new();
    let mut start_idx = 0;

    let gap_continues_episode = |i: usize| -> bool {
        if i + 1 >= tps.len() {
            return false;
        }
        let gap = minutes_between(tps[i].at, tps[i + 1].at);
        if gap <= th.flow_gap_minutes {
            return true;
        }
        // waiting-on-AI rule: gap > flow_gap but ≤ idle, and an AI span
        // from the earlier touchpoint's session covers ≥ half
        if gap <= th.idle_minutes {
            if let Some(ai_end) = tps[i].ai_until {
                let ai_coverage = minutes_between(tps[i].at, ai_end);
                return ai_coverage >= gap / 2.0;
            }
        }
        false
    };

    for i in 0..tps.len() {
        let end_of_run = i + 1 >= tps.len() || !gap_continues_episode(i);
        if end_of_run {
            let span = minutes_between(tps[start_idx].at, tps[i].at);
            if span >= th.flow_min_minutes {
                let mut projects: Vec<String> = Vec::new();
                for tp in &tps[start_idx..=i] {
                    if !projects.contains(&tp.project) {
                        projects.push(tp.project.clone());
                    }
                }
                episodes.push(FlowEpisode {
                    start: tps[start_idx].at,
                    end: tps[i].at,
                    minutes: span,
                    touchpoints: (i - start_idx + 1) as u64,
                    multi_project: projects.len() > 1,
                    projects,
                });
            }
            start_idx = i + 1;
        }
    }

    episodes
}
```

- [ ] **Step 4: Run tests.** `cargo test --test attention_test` then `cargo test`.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Detect flow episodes with waiting-on-AI rule"`

---

### Task 8: Orchestration — raw and attention-intersected overlap

**Files:**
- Modify: `src/analysis/attention.rs`
- Test: `tests/attention_test.rs` (extend)

**Interfaces:**
- Consumes: `&[Touchpoint]`, `&AttentionThresholds`
- Produces:
  ```rust
  pub struct OrchestrationStats {
      pub raw_overlap_minutes: f64,
      pub attended_overlap_minutes: f64,
      pub max_concurrent: u64,
      pub total_ai_minutes: f64,
  }
  pub fn compute_orchestration(tps: &[Touchpoint], th: &AttentionThresholds) -> OrchestrationStats
  ```
  Raw overlap = wall-clock minutes where ≥ 2 sessions had concurrent AI-working spans (`ai_until`). Attended = intersected with user's active attention windows (gap from this touchpoint to the next is ≤ `idle_minutes`). `max_concurrent` = peak simultaneous AI sessions observed.

- [ ] **Step 1: Write the failing tests** (append to `tests/attention_test.rs`):

```rust
#[test]
fn orchestration_detects_concurrent_ai_work() {
    // Two sessions with overlapping AI spans: both working minutes 0–10
    let a = session("a", "/p/a", &[(0, Some(10))]);
    let b = session("b", "/p/b", &[(0, Some(10))]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let orch = compute_orchestration(&tps, &th());
    assert_eq!(orch.raw_overlap_minutes, 10.0);
    assert_eq!(orch.max_concurrent, 2);
}

#[test]
fn orchestration_attended_excludes_walked_away() {
    // User prompts A@0 (AI works 0–20), prompts B@0 (AI works 0–20), then nothing.
    // Raw overlap is 20 min. But the user's only touchpoints are at minute 0,
    // so next-gap is 0 (no next touchpoint) → idle. attended_overlap should be 0
    // (the engagement floor doesn't count as attention span for orchestration).
    let a = session("a", "/p/a", &[(0, Some(20))]);
    let b = session("b", "/p/b", &[(0, Some(20))]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let orch = compute_orchestration(&tps, &th());
    assert_eq!(orch.raw_overlap_minutes, 20.0);
    assert_eq!(orch.attended_overlap_minutes, 0.0);
}

#[test]
fn orchestration_attended_with_active_user() {
    // A@0 (AI 0–10), B@0 (AI 0–10), user prompts A@5 and B@8.
    // User is active: gap 0→5 (5 ≤ 15), 5→8 (3 ≤ 15). Attended window = 0–8.
    // Overlap of two AI spans = 0–10. Intersection with 0–8 = 8 min.
    let a = session("a", "/p/a", &[(0, Some(10)), (5, Some(10))]);
    let b = session("b", "/p/b", &[(0, Some(10)), (8, None)]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let orch = compute_orchestration(&tps, &th());
    assert!(orch.attended_overlap_minutes > 0.0);
    assert!(orch.attended_overlap_minutes <= orch.raw_overlap_minutes);
}
```

- [ ] **Step 2: Run, verify failure.** `compute_orchestration` not found.

- [ ] **Step 3: Implement** in `attention.rs`. The approach: build a list of 1-minute buckets covering the touchpoint window, mark which sessions have AI-working in each bucket, count how many overlap. For the attended intersection, build attention windows (each gap ≤ idle from touchpoint to next). This is O(minutes × sessions) which is fine for weekly personal data.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStats {
    pub raw_overlap_minutes: f64,
    pub attended_overlap_minutes: f64,
    pub max_concurrent: u64,
    pub total_ai_minutes: f64,
}

pub fn compute_orchestration(tps: &[Touchpoint], th: &AttentionThresholds) -> OrchestrationStats {
    if tps.is_empty() {
        return OrchestrationStats {
            raw_overlap_minutes: 0.0,
            attended_overlap_minutes: 0.0,
            max_concurrent: 0,
            total_ai_minutes: 0.0,
        };
    }

    // Collect all AI spans: (start_minute, end_minute, session_id)
    let epoch = tps[0].at;
    let mut ai_spans: Vec<(f64, f64, String)> = Vec::new();
    for tp in tps {
        if let Some(end) = tp.ai_until {
            let start_m = minutes_between(epoch, tp.at);
            let end_m = minutes_between(epoch, end);
            if end_m > start_m {
                ai_spans.push((start_m, end_m, tp.session_id.clone()));
            }
        }
    }

    if ai_spans.is_empty() {
        return OrchestrationStats {
            raw_overlap_minutes: 0.0,
            attended_overlap_minutes: 0.0,
            max_concurrent: 0,
            total_ai_minutes: 0.0,
        };
    }

    let max_end = ai_spans
        .iter()
        .map(|(_, e, _)| *e)
        .fold(0.0f64, f64::max);
    let num_buckets = max_end.ceil() as usize + 1;

    // For each minute bucket, count distinct sessions with AI active
    let mut concurrent = vec![0u64; num_buckets];
    let mut total_ai = 0.0f64;
    for (s, e, _) in &ai_spans {
        let si = (*s).floor() as usize;
        let ei = (*e).ceil() as usize;
        total_ai += e - s;
        for bucket in si..ei.min(num_buckets) {
            concurrent[bucket] += 1;
        }
    }
    // Note: same session can double-count per bucket above. Dedupe per bucket:
    // Simpler: re-do with session sets per bucket.
    let mut bucket_sessions: Vec<HashSet<String>> = vec![HashSet::new(); num_buckets];
    for (s, e, sid) in &ai_spans {
        let si = (*s).floor() as usize;
        let ei = (*e).ceil() as usize;
        for bucket in si..ei.min(num_buckets) {
            bucket_sessions[bucket].insert(sid.clone());
        }
    }

    let max_concurrent = bucket_sessions.iter().map(|s| s.len() as u64).max().unwrap_or(0);
    let raw_overlap: f64 = bucket_sessions
        .iter()
        .filter(|s| s.len() >= 2)
        .count() as f64;

    // Build attended windows from touchpoint gaps
    let mut attended = vec![false; num_buckets];
    for i in 0..tps.len().saturating_sub(1) {
        let gap = minutes_between(tps[i].at, tps[i + 1].at);
        if gap <= th.idle_minutes {
            let si = minutes_between(epoch, tps[i].at).floor() as usize;
            let ei = minutes_between(epoch, tps[i + 1].at).ceil() as usize;
            for bucket in si..ei.min(num_buckets) {
                attended[bucket] = true;
            }
        }
    }

    let attended_overlap: f64 = bucket_sessions
        .iter()
        .enumerate()
        .filter(|(i, s)| s.len() >= 2 && *attended.get(*i).unwrap_or(&false))
        .count() as f64;

    OrchestrationStats {
        raw_overlap_minutes: raw_overlap,
        attended_overlap_minutes: attended_overlap,
        max_concurrent,
        total_ai_minutes: total_ai,
    }
}
```

- [ ] **Step 4: Run tests.** `cargo test --test attention_test` then `cargo test`.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Compute orchestration overlap with attended intersection"`

---

### Task 9: The `AttentionAnalysis` aggregate and `conscience attention` CLI command

**Files:**
- Modify: `src/analysis/attention.rs` (aggregate struct + `analyze` fn)
- Modify: `src/main.rs` (new `Attention` command variant + `run_attention`)
- Create: `src/analysis/attention_report.rs` (terminal rendering)
- Modify: `src/analysis/mod.rs` (add `pub mod attention_report;`)
- Test: full-suite green + manual end-to-end

**Interfaces:**
- Consumes: all functions from Tasks 4–8.
- Produces:
  ```rust
  pub struct AttentionAnalysis {
      pub window_days: u32,
      pub total_touchpoints: u64,
      pub active_time: ActiveTime,
      pub switches_per_day: f64,
      pub dwell: DwellStats,
      pub flow_episodes: Vec<FlowEpisode>,
      pub orchestration: OrchestrationStats,
      pub thresholds_used: AttentionThresholds,
      pub reflection_question: String,
  }
  pub fn analyze_attention(
      sessions: &[AiSession],
      th: &AttentionThresholds,
      days: u32,
      tz: FixedOffset,
  ) -> AttentionAnalysis
  ```
  CLI: `conscience attention [--days 7] [--project <path>] [--json] [--html <path>]`

- [ ] **Step 1: Add the aggregate function** in `attention.rs`:

```rust
pub struct AttentionAnalysis {
    pub window_days: u32,
    pub total_touchpoints: u64,
    pub active_time: ActiveTime,
    pub switches_per_day: f64,
    pub dwell: DwellStats,
    pub flow_episodes: Vec<FlowEpisode>,
    pub orchestration: OrchestrationStats,
    pub thresholds_used: AttentionThresholds,
    pub reflection_question: String,
}

pub fn analyze_attention(
    sessions: &[AiSession],
    th: &AttentionThresholds,
    days: u32,
    tz: FixedOffset,
) -> AttentionAnalysis {
    let since = Utc::now() - chrono::Duration::days(days as i64);
    let tps = collect_touchpoints(sessions, th, since);
    let active_time = compute_active_time(&tps, th, tz);
    let (switches, dwell) = compute_switches_dwell(&tps, th);
    let flow_episodes = compute_flow_episodes(&tps, th);
    let orchestration = compute_orchestration(&tps, th);

    let num_days = active_time.per_day.len().max(1) as f64;
    let switches_per_day = switches as f64 / num_days;

    let reflection_question = generate_attention_reflection(&flow_episodes, &active_time, switches_per_day);

    AttentionAnalysis {
        window_days: days,
        total_touchpoints: tps.len() as u64,
        active_time,
        switches_per_day,
        dwell,
        flow_episodes,
        orchestration,
        thresholds_used: th.clone(),
        reflection_question,
    }
}

fn generate_attention_reflection(
    episodes: &[FlowEpisode],
    active: &ActiveTime,
    switches_per_day: f64,
) -> String {
    let multi = episodes.iter().filter(|e| e.multi_project).count();
    let single = episodes.iter().filter(|e| !e.multi_project).count();
    let n_projects = active.per_project.len();

    if multi > single && n_projects > 1 {
        format!(
            "Most of your flow this period spanned multiple projects ({} multi vs {} single-project episodes across {} projects). \
            Does that feel like richness — different perspectives feeding each other — or fragmentation? \
            What would your ideal week's attention pattern look like?",
            multi, single, n_projects
        )
    } else if switches_per_day > 8.0 {
        format!(
            "You switched projects {:.1} times per day. Is that responsive orchestration, \
            or are you being pulled? Would fewer switches let you go deeper?",
            switches_per_day
        )
    } else if episodes.is_empty() {
        "No flow episodes were detected. Were there stretches that felt like flow but happened \
        outside of AI-assisted work? Is AI usage interrupting flow rather than supporting it?".to_string()
    } else {
        format!(
            "You had {} flow episode(s) this period. Are those the moments that mattered most, \
            or was important work happening in the shorter bursts too?",
            episodes.len()
        )
    }
}
```

- [ ] **Step 2: Create `src/analysis/attention_report.rs`** — terminal renderer:

```rust
use crate::analysis::attention::*;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

pub fn print_attention_analysis(analysis: &AttentionAnalysis) {
    println!();
    println!("  Conscience \u{2014} Attention Analysis ({} day window)", analysis.window_days);
    println!("  Active time estimates are floors based on Claude Code activity only.");
    println!();

    print_project_table(&analysis.active_time.per_project);
    println!();
    print_daily_summary(&analysis.active_time.per_day, analysis.switches_per_day, &analysis.dwell);
    println!();
    print_flow_episodes(&analysis.flow_episodes);
    println!();
    print_orchestration(&analysis.orchestration);
    println!();
    println!("  Reflection");
    println!("  {}", analysis.reflection_question);
    println!();
}

fn print_project_table(projects: &[ProjectAttention]) {
    println!("  Per-Project Active Time (range: earlier \u{2194} later attribution)");
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Project").fg(Color::Cyan),
            Cell::new("Active (min)").fg(Color::Cyan),
            Cell::new("Active (max)").fg(Color::Cyan),
            Cell::new("Touchpoints").fg(Color::Cyan),
            Cell::new("Sessions").fg(Color::Cyan),
        ]);
    for p in projects {
        let name = p.project.rsplit('/').next().unwrap_or(&p.project);
        table.add_row(vec![
            Cell::new(name),
            Cell::new(format!("{:.0}", p.active_minutes_min)),
            Cell::new(format!("{:.0}", p.active_minutes_max)),
            Cell::new(p.touchpoints.to_string()),
            Cell::new(p.sessions.to_string()),
        ]);
    }
    println!("{}", table);
}

fn print_daily_summary(days: &[DayAttention], switches_per_day: f64, dwell: &DwellStats) {
    println!("  Daily Summary");
    for d in days {
        println!(
            "    {} \u{2014} {:.0} min active, {} projects",
            d.date, d.active_minutes, d.projects.len()
        );
    }
    println!();
    println!(
        "  Switches: {:.1}/day \u{2014} Dwell: median {:.0} min, mean {:.0} min, max {:.0} min",
        switches_per_day, dwell.median_minutes, dwell.mean_minutes, dwell.max_minutes
    );
}

fn print_flow_episodes(episodes: &[FlowEpisode]) {
    if episodes.is_empty() {
        println!("  No flow episodes detected (minimum {} min sustained engagement).", 20);
        return;
    }
    println!("  Flow Episodes");
    for (i, ep) in episodes.iter().enumerate() {
        let kind = if ep.multi_project { "multi-project" } else { "single-project" };
        let start = ep.start.format("%m-%d %H:%M");
        println!(
            "    {}. {} \u{2014} {:.0} min, {} touchpoints, {} [{}]",
            i + 1, start, ep.minutes, ep.touchpoints, kind,
            ep.projects.join(", ")
        );
    }
}

fn print_orchestration(orch: &OrchestrationStats) {
    println!("  Orchestration (concurrent AI sessions)");
    println!(
        "    Overlap: {:.0} min (attended: {:.0} min) \u{2014} Peak: {} simultaneous",
        orch.raw_overlap_minutes, orch.attended_overlap_minutes, orch.max_concurrent
    );
}
```

Add `pub mod attention_report;` to `src/analysis/mod.rs`.

- [ ] **Step 3: Wire the CLI command** in `src/main.rs`. Add the variant:

```rust
/// Analyze attention patterns across projects
Attention {
    /// Number of days to look back
    #[arg(long, default_value = "7")]
    days: u32,
    /// Filter to a specific project directory for AI logs
    #[arg(long)]
    project: Option<PathBuf>,
    /// Output as JSON
    #[arg(long)]
    json: bool,
    /// Write an HTML timeline visualization to this path
    #[arg(long)]
    html: Option<PathBuf>,
},
```

Add the match arm:

```rust
Commands::Attention { days, project, json, html } => {
    run_attention(days, project.as_deref(), json, html.as_deref()).await
}
```

And the runner function:

```rust
async fn run_attention(
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
    _html_path: Option<&std::path::Path>,  // Task 10
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = ingest::ai::ingest_claude_code(project)?;
    if summary.session_count == 0 {
        eprintln!("No Claude Code sessions found.");
        std::process::exit(1);
    }

    let cwd = std::env::current_dir()?;
    let manifest_dir = project.unwrap_or(&cwd);
    let manifest = ethics::manifest::Manifest::load(manifest_dir);
    let th = manifest
        .as_ref()
        .map(|m| m.thresholds.attention.clone())
        .unwrap_or_default();

    let tz = chrono::Local::now().fixed_offset().timezone();
    let analysis = analysis::attention::analyze_attention(&summary.sessions, &th, days, tz);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
    } else {
        analysis::attention_report::print_attention_analysis(&analysis);
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests.** `cargo test` — all pass. Then manually:
```bash
cargo run -- attention --days 7
cargo run -- attention --days 7 --json | head -20
```

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Add conscience attention command with terminal stats and JSON output"`

---

### Task 10: HTML timeline visualization

**Files:**
- Create: `src/analysis/attention_html.rs`
- Modify: `src/analysis/mod.rs` (add `pub mod attention_html;`)
- Modify: `src/main.rs` (`run_attention` uses `_html_path` → `html_path`)
- Test: `tests/attention_test.rs` (extend — smoke test)

**Interfaces:**
- Consumes: `&AttentionAnalysis`, `&[Touchpoint]`, `FixedOffset`
- Produces:
  ```rust
  pub fn render_html_timeline(
      analysis: &AttentionAnalysis,
      tps: &[Touchpoint],
      tz: FixedOffset,
  ) -> String
  ```
  Returns a complete self-contained HTML string with inline SVG + CSS. No external assets.

- [ ] **Step 1: Write the failing test** (append to `tests/attention_test.rs`):

```rust
use conscience::analysis::attention_html;

#[test]
fn html_timeline_contains_svg_and_project_lanes() {
    let a = session("a", "/p/alpha", &[(0, Some(5)), (10, Some(15)), (20, None)]);
    let b = session("b", "/p/beta", &[(5, Some(10)), (15, Some(20))]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let tz = FixedOffset::east_opt(0).unwrap();
    let analysis = conscience::analysis::attention::analyze_attention(
        &[
            session("a", "/p/alpha", &[(0, Some(5)), (10, Some(15)), (20, None)]),
            session("b", "/p/beta", &[(5, Some(10)), (15, Some(20))]),
        ],
        &th(),
        30,
        tz,
    );
    let html = attention_html::render_html_timeline(&analysis, &tps, tz);
    assert!(html.contains("<svg"), "must contain SVG");
    assert!(html.contains("alpha"), "project name in lane label");
    assert!(html.contains("beta"), "project name in lane label");
    assert!(html.contains("Attention Timeline"), "title present");
}
```

- [ ] **Step 2: Run, verify failure.** `attention_html` module not found.

- [ ] **Step 3: Implement** `src/analysis/attention_html.rs`. The renderer groups touchpoints by local date, then by project per day. Each day gets an SVG section with swimlanes. The time axis spans the earliest to latest touchpoint of the day, scaled to a fixed SVG width.

```rust
use crate::analysis::attention::*;
use chrono::{FixedOffset, Timelike};
use std::collections::BTreeMap;
use std::fmt::Write;

const SVG_WIDTH: f64 = 900.0;
const LANE_HEIGHT: f64 = 40.0;
const LANE_PAD: f64 = 5.0;
const TICK_R: f64 = 4.0;

pub fn render_html_timeline(
    analysis: &AttentionAnalysis,
    tps: &[Touchpoint],
    tz: FixedOffset,
) -> String {
    let mut html = String::new();
    let _ = writeln!(html, "<title>Attention Timeline</title>");
    let _ = writeln!(html, "<style>");
    let _ = writeln!(html, ":root {{ --bg: #ffffff; --fg: #1a1a1a; --lane-bg: #f5f5f5; --ai-fill: #dbeafe; --tick: #2563eb; --flow-fill: rgba(34,197,94,0.15); }}");
    let _ = writeln!(html, "@media (prefers-color-scheme: dark) {{ :root:not([data-theme=\"light\"]) {{ --bg: #1a1a1a; --fg: #e5e5e5; --lane-bg: #2a2a2a; --ai-fill: #1e3a5f; --tick: #60a5fa; --flow-fill: rgba(34,197,94,0.12); }} }}");
    let _ = writeln!(html, ":root[data-theme=\"dark\"] {{ --bg: #1a1a1a; --fg: #e5e5e5; --lane-bg: #2a2a2a; --ai-fill: #1e3a5f; --tick: #60a5fa; --flow-fill: rgba(34,197,94,0.12); }}");
    let _ = writeln!(html, "body {{ background: var(--bg); color: var(--fg); font-family: system-ui, sans-serif; max-width: 960px; margin: 2rem auto; padding: 0 1rem; }}");
    let _ = writeln!(html, "h1 {{ font-size: 1.4rem; }} h2 {{ font-size: 1.1rem; margin-top: 2rem; }}");
    let _ = writeln!(html, ".lane-label {{ font-size: 0.8rem; fill: var(--fg); }} .time-label {{ font-size: 0.65rem; fill: var(--fg); opacity: 0.6; }}");
    let _ = writeln!(html, "svg {{ display: block; max-width: 100%; overflow-x: auto; }}");
    let _ = writeln!(html, ".legend {{ display: flex; gap: 1.5rem; flex-wrap: wrap; font-size: 0.8rem; margin: 1rem 0; }}");
    let _ = writeln!(html, ".legend-item {{ display: flex; align-items: center; gap: 0.4rem; }}");
    let _ = writeln!(html, ".legend-swatch {{ width: 14px; height: 14px; border-radius: 2px; }}");
    let _ = writeln!(html, "</style>");

    let _ = writeln!(html, "<h1>Attention Timeline</h1>");
    let _ = writeln!(html, "<p>Active time estimates are floors. Per-project ranges reflect attribution uncertainty.</p>");

    let _ = writeln!(html, "<div class=\"legend\">");
    let _ = writeln!(html, "  <div class=\"legend-item\"><div class=\"legend-swatch\" style=\"background:var(--tick)\"></div> Human prompt</div>");
    let _ = writeln!(html, "  <div class=\"legend-item\"><div class=\"legend-swatch\" style=\"background:var(--ai-fill)\"></div> AI working</div>");
    let _ = writeln!(html, "  <div class=\"legend-item\"><div class=\"legend-swatch\" style=\"background:var(--flow-fill);border:1px solid rgba(34,197,94,0.4)\"></div> Flow episode</div>");
    let _ = writeln!(html, "</div>");

    // Group touchpoints by local date
    let mut by_day: BTreeMap<chrono::NaiveDate, Vec<&Touchpoint>> = BTreeMap::new();
    for tp in tps {
        let day = tp.at.with_timezone(&tz).date_naive();
        by_day.entry(day).or_default().push(tp);
    }

    for (date, day_tps) in &by_day {
        let _ = writeln!(html, "<h2>{}</h2>", date);

        // Collect projects in order of first appearance
        let mut projects: Vec<String> = Vec::new();
        for tp in day_tps {
            if !projects.contains(&tp.project) {
                projects.push(tp.project.clone());
            }
        }

        let label_width = 120.0;
        let chart_width = SVG_WIDTH - label_width;
        let total_height = projects.len() as f64 * (LANE_HEIGHT + LANE_PAD) + 30.0;

        let min_hour = day_tps.iter().map(|tp| tp.at.with_timezone(&tz).hour()).min().unwrap_or(0);
        let max_hour = day_tps.iter().map(|tp| {
            let t = tp.at.with_timezone(&tz);
            t.hour() + if t.minute() > 0 { 1 } else { 0 }
        }).max().unwrap_or(24);
        let min_hour = min_hour.max(0);
        let max_hour = (max_hour + 1).min(24);
        let hour_range = (max_hour - min_hour).max(1) as f64;

        let x_for = |dt: chrono::DateTime<FixedOffset>| -> f64 {
            let minutes_since_start = (dt.hour() as f64 - min_hour as f64) * 60.0 + dt.minute() as f64 + dt.second() as f64 / 60.0;
            label_width + (minutes_since_start / (hour_range * 60.0)) * chart_width
        };

        let _ = writeln!(html, "<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", SVG_WIDTH, total_height, SVG_WIDTH, total_height);

        // Time axis labels
        for h in min_hour..=max_hour {
            let x = label_width + ((h - min_hour) as f64 / hour_range) * chart_width;
            let _ = writeln!(html, "  <text x=\"{}\" y=\"12\" class=\"time-label\">{:02}:00</text>", x, h);
        }

        // Lanes
        for (lane_idx, project) in projects.iter().enumerate() {
            let y = 20.0 + lane_idx as f64 * (LANE_HEIGHT + LANE_PAD);
            let name = project.rsplit('/').next().unwrap_or(project);

            // Lane background
            let _ = writeln!(html, "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--lane-bg)\" rx=\"3\"/>",
                label_width, y, chart_width, LANE_HEIGHT);

            // Label
            let _ = writeln!(html, "  <text x=\"{}\" y=\"{}\" class=\"lane-label\">{}</text>",
                5, y + LANE_HEIGHT / 2.0 + 4.0, name);

            // AI spans
            for tp in day_tps.iter().filter(|t| t.project == *project) {
                if let Some(ai_end) = tp.ai_until {
                    let x1 = x_for(tp.at.with_timezone(&tz));
                    let x2 = x_for(ai_end.with_timezone(&tz));
                    if x2 > x1 {
                        let _ = writeln!(html, "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--ai-fill)\" rx=\"2\"/>",
                            x1, y + 2.0, x2 - x1, LANE_HEIGHT - 4.0);
                    }
                }
            }

            // Touchpoint ticks
            for tp in day_tps.iter().filter(|t| t.project == *project) {
                let x = x_for(tp.at.with_timezone(&tz));
                let _ = writeln!(html, "  <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--tick)\"/>",
                    x, y + LANE_HEIGHT / 2.0, TICK_R);
            }
        }

        let _ = writeln!(html, "</svg>");
    }

    // Footer: thresholds
    let _ = writeln!(html, "<hr><p style=\"font-size:0.75rem;opacity:0.6\">");
    let _ = writeln!(html, "Thresholds: idle={}min, floor={}min, flow_gap={}min, flow_min={}min",
        analysis.thresholds_used.idle_minutes,
        analysis.thresholds_used.engagement_floor_minutes,
        analysis.thresholds_used.flow_gap_minutes,
        analysis.thresholds_used.flow_min_minutes);
    let _ = writeln!(html, "</p>");

    html
}
```

Add `pub mod attention_html;` in `src/analysis/mod.rs`.

- [ ] **Step 4: Wire `--html` in `run_attention`** — change `_html_path` to `html_path`, add after the print/json block:

```rust
if let Some(path) = html_path {
    // Re-collect touchpoints for the HTML renderer
    let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let tps = analysis::attention::collect_touchpoints(&summary.sessions, &th, since);
    let html = analysis::attention_html::render_html_timeline(&analysis, &tps, tz);
    std::fs::write(path, &html)?;
    eprintln!("Timeline written to {}", path.display());
}
```

- [ ] **Step 5: Run tests.** `cargo test` — all pass including the HTML smoke test. Then manual end-to-end:

```bash
cargo run -- attention --days 7 --html /tmp/attention.html
open /tmp/attention.html   # verify in browser
```

- [ ] **Step 6: Update docs.** In `CLAUDE.md`, add the `attention` command to the Commands section. In `README.md`, add a section after "Team Reflection":

```markdown
### Attention & Flow

Analyze how your time and attention move across projects:

\```
# Weekly attention summary (default 7 days)
conscience attention

# With an HTML timeline visualization
conscience attention --days 14 --html attention.html

# JSON for scripting
conscience attention --json
\```
```

- [ ] **Step 7: Commit.** `git add -A && git commit -m "Add HTML timeline visualization for attention analysis"`

---

## Self-Review Results

**Spec coverage check:**
- Corrected turn counting → Task 1 ✓
- Interaction extraction (human_at, ai_until, uuid) → Task 2 ✓
- Cross-file dedup → Task 4 ✓
- Project aliases → Task 4 ✓
- Touchpoint-level windowing → Task 4 ✓
- Active time with attribution ranges → Task 5 ✓
- Local-timezone day bucketing → Task 5 ✓
- Context switches and dwell → Task 6 ✓
- Flow episodes with waiting-on-AI rule → Task 7 ✓
- Orchestration (raw + attended) → Task 8 ✓
- `AttentionThresholds` in conscience.yaml → Task 3 ✓
- Ratio recalibration → Task 3 ✓
- `conscience attention` CLI command → Task 9 ✓
- Terminal stats output → Task 9 ✓
- JSON output → Task 9 ✓
- HTML timeline → Task 10 ✓
- Reflection integration (anti-Goodhart) → Task 9 (reflection_question in analysis + print) ✓
- Privacy (timestamps + paths only) → global constraint ✓
- Personal-scope standing constraint → documented in spec, no code needed ✓
- Honest-measurement caveats in output → Task 9 (header text) ✓

**Placeholder scan:** No TBD, TODO, or "implement later" found.

**Type consistency:** `AttentionThresholds`, `Touchpoint`, `ActiveTime`, `ProjectAttention`, `DayAttention`, `DwellStats`, `FlowEpisode`, `OrchestrationStats`, `AttentionAnalysis` — names and field types are consistent across all tasks. `minutes_between` helper introduced in Task 5, used in Tasks 5–8. `collect_touchpoints` introduced in Task 4, used in Tasks 5–10.