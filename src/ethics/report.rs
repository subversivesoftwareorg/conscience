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
