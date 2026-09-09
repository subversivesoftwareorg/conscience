use crate::ai_tools::models::AiSession;
use crate::ethics::manifest::AttentionThresholds;
use chrono::{DateTime, FixedOffset, Utc};
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
        let project = apply_alias(
            s.project_path.as_deref().unwrap_or("(unknown)"),
            &th.project_aliases,
        );
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DwellStats {
    pub median_minutes: f64,
    pub mean_minutes: f64,
    pub max_minutes: f64,
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

        *earlier.entry(tp.project.clone()).or_insert(0.0) += credit;
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

    ActiveTime {
        total_minutes: total,
        per_project,
        per_day,
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStats {
    pub raw_overlap_minutes: f64,
    pub attended_overlap_minutes: f64,
    pub max_concurrent: u64,
    pub total_ai_minutes: f64,
}

pub fn compute_orchestration(
    tps: &[Touchpoint],
    th: &AttentionThresholds,
) -> OrchestrationStats {
    if tps.is_empty() {
        return OrchestrationStats {
            raw_overlap_minutes: 0.0,
            attended_overlap_minutes: 0.0,
            max_concurrent: 0,
            total_ai_minutes: 0.0,
        };
    }

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

    let mut total_ai = 0.0f64;
    let mut bucket_sessions: Vec<HashSet<String>> = vec![HashSet::new(); num_buckets];
    for (s, e, sid) in &ai_spans {
        total_ai += e - s;
        let si = (*s).floor() as usize;
        let ei = (*e).ceil() as usize;
        for bucket in si..ei.min(num_buckets) {
            bucket_sessions[bucket].insert(sid.clone());
        }
    }

    let max_concurrent = bucket_sessions
        .iter()
        .map(|s| s.len() as u64)
        .max()
        .unwrap_or(0);
    let raw_overlap: f64 = bucket_sessions
        .iter()
        .filter(|s| s.len() >= 2)
        .count() as f64;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            Does that feel like richness \u{2014} different perspectives feeding each other \u{2014} or fragmentation? \
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
        DwellStats {
            median_minutes: 0.0,
            mean_minutes: 0.0,
            max_minutes: 0.0,
        }
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
