use crate::analysis::energy::*;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

pub fn print_energy_report(estimate: &EnergyEstimate) {
    println!();
    println!(
        "  Conscience \u{2014} Energy Estimate ({} day window)",
        estimate.period_days
    );
    println!("  Based on third-party research. No provider publishes official energy data.");
    println!("  All figures are estimates with stated uncertainty ranges.");
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Model").fg(Color::Cyan),
            Cell::new("Tier").fg(Color::Cyan),
            Cell::new("Sessions").fg(Color::Cyan),
            Cell::new("Output Tokens").fg(Color::Cyan),
            Cell::new("Est. Wh").fg(Color::Cyan),
            Cell::new("Uncertainty").fg(Color::Cyan),
        ]);

    for m in &estimate.per_model {
        let tokens = if m.tokens.output >= 1_000_000 {
            format!("{:.1}M", m.tokens.output as f64 / 1_000_000.0)
        } else if m.tokens.output >= 1_000 {
            format!("{:.1}K", m.tokens.output as f64 / 1_000.0)
        } else {
            m.tokens.output.to_string()
        };

        table.add_row(vec![
            Cell::new(&m.model),
            Cell::new(&m.tier),
            Cell::new(m.sessions.to_string()),
            Cell::new(&tokens),
            Cell::new(format!("{:.1}", m.total_wh)),
            Cell::new(format!("\u{00b1}{:.0}%", m.uncertainty_pct)),
        ]);
    }
    println!("{}", table);

    println!();
    println!("  Totals");
    println!(
        "    Estimated energy: {:.1} Wh (range: {:.0}\u{2013}{:.0} Wh)",
        estimate.total_wh, estimate.uncertainty_range.0, estimate.uncertainty_range.1
    );
    println!(
        "    Equivalent to: ~{:.1} hours of laptop use (60W)",
        estimate.total_wh / 60.0
    );
    if estimate.total_wh > 0.0 {
        println!(
            "    Blended efficiency: {:.2} Wh per 1K output tokens",
            estimate.blended_wh_per_1k_output
        );
    }
    if let Some(co2) = estimate.co2_kg {
        let intensity = estimate.grid_carbon_intensity.unwrap_or(0.42);
        println!(
            "    Estimated CO2: {:.3} kg (at {:.2} kgCO2/kWh)",
            co2, intensity
        );
    }
    println!();
    println!("  Sources: {}", estimate.methodology);
    println!();
}
