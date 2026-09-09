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
