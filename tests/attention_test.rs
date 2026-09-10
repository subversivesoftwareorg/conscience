use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use conscience::ai_tools::models::*;
use conscience::analysis::attention::*;
use conscience::analysis::attention_html;
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

fn utc_tz() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

#[test]
fn per_day_switches_populated() {
    // A@0 -> B@10: different project, gap 10 ≤ idle 15 -> 1 switch on the single day.
    let a = session("a", "/p/a", &[(0, None)]);
    let b = session("b", "/p/b", &[(10, None)]);
    let tps = collect_touchpoints(&[a, b], &th(), long_ago());
    let at = compute_active_time(&tps, &th(), utc_tz());
    assert_eq!(at.per_day.len(), 1);
    assert_eq!(at.per_day[0].switches, 1, "A->B is one context switch");
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
        3650,
        tz,
    );
    let html = attention_html::render_html_timeline(&analysis, &tps, tz);
    assert!(html.contains("<svg"), "must contain SVG");
    assert!(html.contains("alpha"), "project name in lane label");
    assert!(html.contains("beta"), "project name in lane label");
    assert!(html.contains("Attention Timeline"), "title present");
}
