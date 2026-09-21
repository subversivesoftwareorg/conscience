use crate::analysis::comparisons::{Comparisons, per_day};
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
    if estimate.excluded_sessions > 0 {
        println!(
            "  {} session(s) produced no model output (API errors, rate limits, empty sessions) and are excluded.",
            estimate.excluded_sessions
        );
    }
    if estimate.per_model.iter().any(|m| m.tier.contains("assumed")) {
        println!(
            "  Tiers marked (assumed) have no published measurement; they are treated as GPT-4-class with doubled uncertainty."
        );
    }

    println!();
    println!("  Totals");
    println!(
        "    Estimated energy: {:.1} Wh (range: {:.0}\u{2013}{:.0} Wh)",
        estimate.total_wh, estimate.uncertainty_range.0, estimate.uncertainty_range.1
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
    if let Some(water) = estimate.water_liters {
        let rate = estimate.water_liters_per_kwh.unwrap_or(1.8);
        if water >= 1.0 {
            println!(
                "    Estimated water: {:.1} liters (at {:.1} L/kWh)",
                water, rate
            );
        } else {
            println!(
                "    Estimated water: {:.0} mL (at {:.1} L/kWh)",
                water * 1000.0, rate
            );
        }
    }

    // Per-day rates make a 7-day and a 30-day report comparable.
    let daily = per_day(
        estimate.period_days,
        estimate.total_wh,
        estimate.co2_kg,
        estimate.water_liters,
    );
    if estimate.period_days > 0 && estimate.total_wh > 0.0 {
        let mut parts = vec![format!("{:.0} Wh", daily.wh)];
        if let Some(c) = daily.co2_kg {
            parts.push(format!("{:.3} kg CO2", c));
        }
        if let Some(w) = daily.water_liters {
            parts.push(format!("{:.1} L water", w));
        }
        println!(
            "    Per day over {} days: {}",
            daily.days,
            parts.join(", ")
        );
    }

    print_comparisons(&estimate.comparisons);

    println!("  Sources: {}", estimate.methodology);
    println!();
}

/// The "equivalent to" block: chosen by scale, each line naming its source.
pub fn print_comparisons(c: &Comparisons) {
    if c.is_empty() {
        return;
    }
    println!();
    println!("  Equivalent to (illustrative)");
    for (label, list) in [("Energy", &c.energy), ("CO2", &c.co2), ("Water", &c.water)] {
        for cmp in list {
            println!("    {:<7} \u{2248} {}", label, cmp.phrase());
            println!("            source: {}", cmp.source);
        }
    }
    println!();
    for note in &c.notes {
        println!("  Note: {}", note);
    }
    println!();
}
