use crate::ai_tools::models::AiUsageSummary;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

pub fn print_ai_summary(summary: &AiUsageSummary) {
    println!();
    println!("  {} Usage Summary", summary.tool);
    println!(
        "  {} session(s) found",
        summary.session_count
    );
    println!();

    print_token_table(summary);
    println!();
    print_turns_table(summary);
    println!();
    print_tools_table(summary);
    println!();
    print_models_table(summary);

    if summary.unique_files_touched > 0 {
        println!();
        print_files_table(summary);
    }

    if !summary.sessions.is_empty() {
        println!();
        print_sessions_table(summary);
    }
}

fn print_token_table(summary: &AiUsageSummary) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Token Metric").fg(Color::Cyan),
            Cell::new("Count").fg(Color::Cyan),
        ]);

    table.add_row(vec![
        "Input Tokens".to_string(),
        format_number(summary.total_tokens.input),
    ]);
    table.add_row(vec![
        "Output Tokens".to_string(),
        format_number(summary.total_tokens.output),
    ]);
    table.add_row(vec![
        "Cache Creation".to_string(),
        format_number(summary.total_tokens.cache_creation),
    ]);
    table.add_row(vec![
        "Cache Read".to_string(),
        format_number(summary.total_tokens.cache_read),
    ]);
    table.add_row(vec![
        "Total".to_string(),
        format_number(summary.total_tokens.total()),
    ]);

    println!("  Tokens");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_turns_table(summary: &AiUsageSummary) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Turn Metric").fg(Color::Cyan),
            Cell::new("Count").fg(Color::Cyan),
        ]);

    table.add_row(vec![
        "Human Turns".to_string(),
        summary.total_turns.human.to_string(),
    ]);
    table.add_row(vec![
        "Assistant Turns".to_string(),
        summary.total_turns.assistant.to_string(),
    ]);

    let ratio = if summary.total_turns.human > 0 {
        format!(
            "{:.1}:1",
            summary.total_turns.assistant as f64 / summary.total_turns.human as f64
        )
    } else {
        "N/A".to_string()
    };
    table.add_row(vec!["AI:Human Ratio".to_string(), ratio]);

    println!("  Conversation");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_tools_table(summary: &AiUsageSummary) {
    if summary.tools_used.is_empty() {
        return;
    }

    let mut sorted: Vec<_> = summary.tools_used.iter().collect();
    sorted.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Tool").fg(Color::Cyan),
            Cell::new("Invocations").fg(Color::Cyan),
        ]);

    for (tool, count) in &sorted {
        table.add_row(vec![tool.to_string(), count.to_string()]);
    }

    println!("  Tools Used");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_models_table(summary: &AiUsageSummary) {
    if summary.models_used.is_empty() {
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Model").fg(Color::Cyan),
            Cell::new("Sessions").fg(Color::Cyan),
        ]);

    let mut sorted: Vec<_> = summary.models_used.iter().collect();
    sorted.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));

    for (model, count) in &sorted {
        table.add_row(vec![model.to_string(), count.to_string()]);
    }

    println!("  Models");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_files_table(summary: &AiUsageSummary) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Files Metric").fg(Color::Cyan),
            Cell::new("Count").fg(Color::Cyan),
        ]);

    table.add_row(vec![
        "Total File Operations".to_string(),
        summary.files_touched_count.to_string(),
    ]);
    table.add_row(vec![
        "Unique Files".to_string(),
        summary.unique_files_touched.to_string(),
    ]);

    println!("  Files");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_sessions_table(summary: &AiUsageSummary) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Session").fg(Color::Cyan),
            Cell::new("Started").fg(Color::Cyan),
            Cell::new("Turns").fg(Color::Cyan),
            Cell::new("Output Tokens").fg(Color::Cyan),
            Cell::new("Branch").fg(Color::Cyan),
        ]);

    let mut sessions: Vec<_> = summary.sessions.iter().collect();
    sessions.sort_by_key(|s| s.started_at);

    for session in &sessions {
        let started = session
            .started_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        table.add_row(vec![
            session.session_id[..8].to_string(),
            started,
            session.turns.total.to_string(),
            format_number(session.tokens.output),
            session
                .git_branch
                .clone()
                .unwrap_or_else(|| "-".to_string()),
        ]);
    }

    println!("  Sessions");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
