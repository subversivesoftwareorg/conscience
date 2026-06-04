use crate::github::models::RepoSummary;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use std::collections::HashMap;

pub fn print_summary(summary: &RepoSummary) {
    println!();
    println!(
        "  {} / {}",
        summary.owner, summary.repo
    );
    println!(
        "  {} to {}",
        summary.period_start.format("%Y-%m-%d"),
        summary.period_end.format("%Y-%m-%d")
    );
    println!();

    print_commit_summary(summary);
    println!();
    print_pr_summary(summary);
    println!();
    print_contributor_summary(summary);
}

fn print_commit_summary(summary: &RepoSummary) {
    let total = summary.commits.len();
    let unique_authors: std::collections::HashSet<_> =
        summary.commits.iter().map(|c| &c.author).collect();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Metric").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ]);

    table.add_row(vec!["Total Commits", &total.to_string()]);
    table.add_row(vec!["Unique Authors", &unique_authors.len().to_string()]);

    println!("  Commits");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_pr_summary(summary: &RepoSummary) {
    let total = summary.pull_requests.len();
    let merged: Vec<_> = summary
        .pull_requests
        .iter()
        .filter(|pr| pr.merged_at.is_some())
        .collect();
    let open: Vec<_> = summary
        .pull_requests
        .iter()
        .filter(|pr| pr.state == "open")
        .collect();

    let avg_time_to_merge = if !merged.is_empty() {
        let total_hours: f64 = merged
            .iter()
            .filter_map(|pr| pr.time_to_merge_hours)
            .sum();
        let count = merged
            .iter()
            .filter(|pr| pr.time_to_merge_hours.is_some())
            .count();
        if count > 0 {
            Some(total_hours / count as f64)
        } else {
            None
        }
    } else {
        None
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Metric").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ]);

    table.add_row(vec!["Total PRs", &total.to_string()]);
    table.add_row(vec!["Merged", &merged.len().to_string()]);
    table.add_row(vec!["Open", &open.len().to_string()]);
    if let Some(hours) = avg_time_to_merge {
        if hours < 24.0 {
            table.add_row(vec!["Avg Time to Merge", &format!("{:.1} hours", hours)]);
        } else {
            table.add_row(vec![
                "Avg Time to Merge",
                &format!("{:.1} days", hours / 24.0),
            ]);
        }
    }

    println!("  Pull Requests");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}

fn print_contributor_summary(summary: &RepoSummary) {
    let mut contributions: HashMap<&str, (usize, usize)> = HashMap::new();

    for commit in &summary.commits {
        contributions
            .entry(&commit.author)
            .or_insert((0, 0))
            .0 += 1;
    }
    for pr in &summary.pull_requests {
        contributions.entry(&pr.author).or_insert((0, 0)).1 += 1;
    }

    let mut sorted: Vec<_> = contributions.iter().collect();
    sorted.sort_by_key(|&(_, (commits, prs))| std::cmp::Reverse(commits + prs));

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Contributor").fg(Color::Cyan),
            Cell::new("Commits").fg(Color::Cyan),
            Cell::new("PRs").fg(Color::Cyan),
        ]);

    for (author, (commits, prs)) in sorted.iter().take(15) {
        table.add_row(vec![
            author.to_string(),
            commits.to_string(),
            prs.to_string(),
        ]);
    }

    println!("  Contributors (top 15)");
    for line in table.to_string().lines() {
        println!("  {}", line);
    }
}
