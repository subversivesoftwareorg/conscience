use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
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
