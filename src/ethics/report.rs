use crate::ethics::models::*;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

pub fn print_ethical_analysis(analysis: &EthicalAnalysis) {
    println!();
    println!("  Conscience \u{2014} Ethical Analysis");
    println!();

    print_signals(&analysis.signals);
    println!();
    print_scorecard(&analysis.scorecard);
    println!();
    print_reflections(&analysis.reflections);
}

fn print_signals(signals: &[Signal]) {
    if signals.is_empty() {
        println!("  No signals detected (insufficient data).");
        return;
    }

    println!("  Signals");
    println!();

    let mut sorted = signals.to_vec();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.severity));

    for signal in &sorted {
        let color = match signal.severity {
            Severity::Warning => "\x1b[31m",
            Severity::Concern => "\x1b[33m",
            Severity::Info => "\x1b[36m",
            Severity::Healthy => "\x1b[32m",
        };
        let reset = "\x1b[0m";

        println!(
            "  {}{:>7}{} {} [{}]",
            color,
            signal.severity.to_string(),
            reset,
            signal.title,
            signal.principle.name()
        );
        println!("          {}", signal.detail);
        println!("          Evidence: {}", signal.evidence);
        println!();
    }
}

fn print_scorecard(scorecard: &[DimensionScore]) {
    println!("  Scorecard");
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Principle").fg(Color::Cyan),
            Cell::new("Signals").fg(Color::Cyan),
            Cell::new("Status").fg(Color::Cyan),
        ]);

    for dim in scorecard {
        let signal_count = dim.auto_signals.len();
        let worst_severity = dim
            .auto_signals
            .iter()
            .map(|s| s.severity)
            .max()
            .unwrap_or(Severity::Healthy);

        let status = if dim.needs_human_input && signal_count == 0 {
            "Needs human input".to_string()
        } else if dim.needs_human_input {
            format!("{} (+ needs human input)", worst_severity)
        } else if signal_count == 0 {
            "No data".to_string()
        } else {
            worst_severity.to_string()
        };

        table.add_row(vec![
            dim.principle.name().to_string(),
            signal_count.to_string(),
            status,
        ]);
    }

    for line in table.to_string().lines() {
        println!("  {}", line);
    }

    println!();
    println!("  Source principles:");
    for dim in scorecard {
        if !dim.auto_signals.is_empty() || dim.needs_human_input {
            println!("    {} \u{2014} {}", dim.principle.name(), dim.principle.source());
        }
    }
}

fn print_reflections(reflections: &[ReflectionQuestion]) {
    println!();
    print!("{}", render_reflection_session(reflections));
}

/// Render reflection questions as a retrospective-friendly session,
/// used by both `examine` and the standalone `reflect` command.
pub fn render_reflection_session(reflections: &[ReflectionQuestion]) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "  Reflection Questions");
    let _ = writeln!(
        out,
        "  For team discussion \u{2014} not automated judgment."
    );
    let _ = writeln!(out);

    for (i, q) in reflections.iter().enumerate() {
        let _ = writeln!(out, "  {}. {}", i + 1, q.principle.name());
        let _ = writeln!(out, "     Source: {}", q.principle.source());
        let _ = writeln!(out, "     Data: {}", q.data_context);
        let _ = writeln!(out, "     Q: {}", q.question);
        let _ = writeln!(out);
    }

    out
}

pub fn print_multi_project_analysis(analysis: &MultiProjectAnalysis) {
    println!();
    println!("  Conscience \u{2014} Cross-Project Analysis");
    println!(
        "  {} projects, {} sessions, {}K output tokens",
        analysis.total_projects,
        analysis.total_sessions,
        analysis.total_output_tokens / 1_000
    );
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Project").fg(Color::Cyan),
            Cell::new("Sessions").fg(Color::Cyan),
            Cell::new("Tokens").fg(Color::Cyan),
            Cell::new("AI:Human").fg(Color::Cyan),
            Cell::new("Signals").fg(Color::Cyan),
            Cell::new("Worst").fg(Color::Cyan),
        ]);

    let mut sorted = analysis.projects.clone();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.total_output_tokens));

    for project in &sorted {
        let signal_count = project.analysis.signals.len();
        let worst = project
            .analysis
            .signals
            .iter()
            .map(|s| s.severity)
            .max()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".to_string());

        let tokens = if project.total_output_tokens >= 1_000_000 {
            format!("{:.1}M", project.total_output_tokens as f64 / 1_000_000.0)
        } else if project.total_output_tokens >= 1_000 {
            format!("{:.1}K", project.total_output_tokens as f64 / 1_000.0)
        } else {
            project.total_output_tokens.to_string()
        };

        table.add_row(vec![
            project
                .project_name
                .as_deref()
                .unwrap_or("unknown")
                .to_string(),
            project.session_count.to_string(),
            tokens,
            format!("{:.1}:1", project.ai_human_ratio),
            signal_count.to_string(),
            worst,
        ]);
    }

    for line in table.to_string().lines() {
        println!("  {}", line);
    }

    if !analysis.outlier_signals.is_empty() {
        println!();
        println!("  Cross-Project Outliers");
        println!();
        print_signals(&analysis.outlier_signals);
    }
}

/// Cross-project analysis as a Markdown digest, for retrospectives and
/// weekly summaries. Pure: the caller decides where it goes.
pub fn render_multi_project_markdown(analysis: &MultiProjectAnalysis, days: u32) -> String {
    let mut md = String::new();

    use std::fmt::Write as _;
    let _ = writeln!(md, "# Conscience Weekly Digest");
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "**Period:** {} days ending {} | **Projects:** {} | **Sessions:** {} | **Output tokens:** {}K",
        days,
        chrono::Utc::now().format("%Y-%m-%d"),
        analysis.total_projects,
        analysis.total_sessions,
        analysis.total_output_tokens / 1_000,
    );

    // Rough energy from total output tokens using large-tier default (1.5 Wh/1K output)
    let rough_energy_wh = analysis.total_output_tokens as f64 * 1.5 / 1000.0;
    let energy_config = crate::ethics::manifest::EnergyConfig::default();
    let comparisons = crate::analysis::comparisons::Comparisons::for_estimate(
        rough_energy_wh,
        (rough_energy_wh * 0.5, rough_energy_wh * 1.5),
        energy_config.grid_carbon_intensity.map(|gi| rough_energy_wh / 1000.0 * gi),
        energy_config.water_liters_per_kwh.map(|wl| rough_energy_wh / 1000.0 * wl),
        &energy_config,
    );
    // One equivalent per quantity keeps the headline to a single line.
    let equivalents: Vec<String> = [&comparisons.energy, &comparisons.co2, &comparisons.water]
        .iter()
        .filter_map(|list| list.first())
        .map(|c| c.phrase())
        .collect();
    let _ = writeln!(
        md,
        "**Estimated energy:** ~{:.0} Wh (\u{00b1}50%), roughly {}",
        rough_energy_wh,
        if equivalents.is_empty() {
            "nothing measurable".to_string()
        } else {
            equivalents.join("; ")
        }
    );
    let _ = writeln!(md);

    // Top Concerns
    let mut concerns: Vec<(&str, &Signal)> = Vec::new();
    for project in &analysis.projects {
        let name = project.project_name.as_deref().unwrap_or("unknown");
        for signal in &project.analysis.signals {
            if signal.severity >= Severity::Concern {
                concerns.push((name, signal));
            }
        }
    }
    for signal in &analysis.outlier_signals {
        if signal.severity >= Severity::Concern {
            concerns.push(("(cross-project)", signal));
        }
    }
    concerns.sort_by_key(|(_, s)| std::cmp::Reverse(s.severity));

    if concerns.is_empty() {
        let _ = writeln!(md, "## No Concerns");
        let _ = writeln!(md, "All projects are in healthy territory this period.");
    } else {
        let _ = writeln!(md, "## Top Concerns ({} signals)", concerns.len());
        let _ = writeln!(md);
        for (project, signal) in concerns.iter().take(10) {
            let _ = writeln!(
                md,
                "- **{}** {} — {} [{}]",
                signal.severity,
                project,
                signal.title,
                signal.principle.name()
            );
            let _ = writeln!(md, "  {}", signal.detail);
        }
        if concerns.len() > 10 {
            let _ = writeln!(md, "- ...and {} more", concerns.len() - 10);
        }
    }
    let _ = writeln!(md);

    // Healthy Patterns
    let healthy: Vec<(&str, &Signal)> = analysis
        .projects
        .iter()
        .flat_map(|p| {
            let name = p.project_name.as_deref().unwrap_or("unknown");
            p.analysis
                .signals
                .iter()
                .filter(|s| s.severity == Severity::Healthy)
                .map(move |s| (name, s))
        })
        .collect();

    if !healthy.is_empty() {
        let _ = writeln!(md, "## Healthy Patterns");
        let _ = writeln!(md);
        for (project, signal) in healthy.iter().take(5) {
            let _ = writeln!(md, "- **{}** — {} [{}]", project, signal.title, signal.principle.name());
        }
        if healthy.len() > 5 {
            let _ = writeln!(md, "- ...and {} more across projects", healthy.len() - 5);
        }
        let _ = writeln!(md);
    }

    // Per-project one-liners
    let _ = writeln!(md, "## Project Summary");
    let _ = writeln!(md);
    let _ = writeln!(md, "| Project | Sessions | Tokens | Signals | Worst |");
    let _ = writeln!(md, "|---------|----------|--------|---------|-------|");
    for p in &analysis.projects {
        let name = p.project_name.as_deref().unwrap_or("unknown");
        let worst = p
            .analysis
            .signals
            .iter()
            .map(|s| s.severity)
            .max()
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "—".to_string());
        let _ = writeln!(
            md,
            "| {} | {} | {}K | {} | {} |",
            name,
            p.session_count,
            p.total_output_tokens / 1_000,
            p.analysis.signals.len(),
            worst,
        );
    }
    let _ = writeln!(md);

    // Reflection
    let _ = writeln!(md, "## Reflection");
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "Looking at the past {} days across {} projects: is the AI compute ({} sessions, \
        ~{:.0} Wh estimated energy) proportionate to the value delivered? \
        Are the concerns above worth investigating, or are they noise?",
        days,
        analysis.total_projects,
        analysis.total_sessions,
        rough_energy_wh,
    );

    md
}

/// The default `examine` view: what deserves attention and what is worth
/// discussing. Everything else is behind `--full`.
pub fn print_examine_brief(snapshot: &crate::snapshot::Snapshot) {
    let analysis = &snapshot.analysis;
    println!();
    println!("  Conscience \u{2014} Ethical Analysis");
    println!();

    let mut attention: Vec<&Signal> = analysis
        .signals
        .iter()
        .filter(|s| s.severity >= Severity::Concern)
        .collect();
    attention.sort_by_key(|s| std::cmp::Reverse(s.severity));

    println!("  Deserves attention");
    println!();
    if attention.is_empty() {
        // Distinct from "healthy": absence of concerns in what we could see.
        println!("  No concerns or warnings detected in the available data.");
        println!();
    } else {
        for s in &attention {
            let color = match s.severity {
                Severity::Warning => "\x1b[31m",
                _ => "\x1b[33m",
            };
            println!(
                "  {}{:>7}\x1b[0m {} [{}]",
                color,
                s.severity.to_string(),
                s.title,
                s.principle.name()
            );
            println!("          {}", s.detail);
            println!();
        }
    }

    // One question: for the principle with the most serious signals, else
    // the first question we have.
    let focus = attention.first().map(|s| s.principle);
    let question = focus
        .and_then(|p| analysis.reflections.iter().find(|r| r.principle == p))
        .or_else(|| analysis.reflections.first());
    if let Some(q) = question {
        println!("  Worth discussing");
        println!();
        println!("  {} \u{2014} {}", q.principle.name(), q.principle.source());
        println!("     Data: {}", q.data_context);
        println!("     Q: {}", q.question);
        println!();
    }

    let healthy = analysis
        .signals
        .iter()
        .filter(|s| s.severity < Severity::Concern)
        .count();
    println!(
        "  {} more signal(s) at info or healthy level; run with --full for all signals, the scorecard, and every reflection question.",
        healthy
    );
    println!();
}
