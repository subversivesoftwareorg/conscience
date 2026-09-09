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
