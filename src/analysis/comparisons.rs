//! Everyday comparisons for energy, CO2, and water estimates.
//!
//! "21 kWh" means little to most people; "about 0.7 days of a US
//! household's electricity" does. Each comparison is one widely cited
//! reference figure with its source named, overridable in `conscience.yaml`
//! so a team can swap in regional values. Comparisons are chosen by scale:
//! a single session reads best in phone charges, a month across projects
//! in household-days. They are illustrative, never a score, and they carry
//! the estimate's uncertainty range with them.

use crate::ethics::manifest::{ComparisonOverride, EnergyConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Which estimated quantity a comparison translates. Base units are
/// watt-hours, kilograms of CO2, and liters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quantity {
    Energy,
    Co2,
    Water,
}

/// One reference figure: how much of the base unit one "thing" is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reference {
    pub key: String,
    pub quantity: Quantity,
    /// Base units (Wh, kg, or L) per one unit of the comparison.
    pub value: f64,
    pub singular: String,
    pub plural: String,
    pub source: String,
}

fn r(
    key: &str,
    quantity: Quantity,
    value: f64,
    singular: &str,
    plural: &str,
    source: &str,
) -> Reference {
    Reference {
        key: key.into(),
        quantity,
        value,
        singular: singular.into(),
        plural: plural.into(),
        source: source.into(),
    }
}

/// The built-in table. Values are deliberately round, widely cited
/// figures; the source column is what the output shows.
pub fn default_references() -> Vec<Reference> {
    use Quantity::*;
    vec![
        r(
            "phone_charge",
            Energy,
            15.0,
            "full phone charge",
            "full phone charges",
            "typical smartphone battery, ~15 Wh",
        ),
        r(
            "laptop_hour",
            Energy,
            60.0,
            "hour of laptop use",
            "hours of laptop use",
            "60 W laptop",
        ),
        r(
            "us_household_day",
            Energy,
            29_600.0,
            "day of a US household's electricity",
            "days of a US household's electricity",
            "EIA 2022, 10,791 kWh per household per year",
        ),
        r(
            "eu_household_day",
            Energy,
            10_000.0,
            "day of an EU household's electricity",
            "days of an EU household's electricity",
            "Eurostat, ~3,600 kWh per household per year",
        ),
        r(
            "ev_mile",
            Energy,
            280.0,
            "mile of electric-car driving",
            "miles of electric-car driving",
            "EPA combined fleet average, ~0.28 kWh per mile",
        ),
        r(
            "car_mile",
            Co2,
            0.40,
            "mile of gasoline-car driving",
            "miles of gasoline-car driving",
            "EPA average passenger vehicle, 404 g CO2 per mile",
        ),
        r(
            "us_person_electricity_day",
            Co2,
            12.4,
            "day of one US person's electricity emissions",
            "days of one US person's electricity emissions",
            "29.6 kWh per day at 0.42 kg CO2 per kWh",
        ),
        r(
            "bottle",
            Water,
            0.5,
            "bottle of water",
            "bottles of water",
            "500 mL bottle",
        ),
        r(
            "shower",
            Water,
            65.0,
            "shower",
            "showers",
            "EPA WaterSense, 8 minutes at 2.1 gallons per minute",
        ),
        r(
            "drinking_day",
            Water,
            2.0,
            "day of one person's drinking water",
            "days of one person's drinking water",
            "2 L per day",
        ),
    ]
}

/// The built-in table with a team's overrides applied: values, wording,
/// or sources replaced, entries disabled, and new entries added.
pub fn references_with(overrides: &BTreeMap<String, ComparisonOverride>) -> Vec<Reference> {
    let mut refs = default_references();
    for (key, ov) in overrides {
        if let Some(existing) = refs.iter_mut().find(|r| &r.key == key) {
            if let Some(v) = ov.value {
                existing.value = v;
            }
            if let Some(s) = &ov.singular {
                existing.singular = s.clone();
            }
            if let Some(p) = &ov.plural {
                existing.plural = p.clone();
            }
            if let Some(s) = &ov.source {
                existing.source = s.clone();
            }
        } else if let (Some(q), Some(v), Some(p)) = (ov.quantity, ov.value, &ov.plural) {
            refs.push(Reference {
                key: key.clone(),
                quantity: q,
                value: v,
                singular: ov.singular.clone().unwrap_or_else(|| p.clone()),
                plural: p.clone(),
                source: ov.source.clone().unwrap_or_else(|| "team-provided".into()),
            });
        }
    }
    refs.retain(|r| {
        !overrides
            .get(&r.key)
            .and_then(|o| o.disabled)
            .unwrap_or(false)
            && r.value > 0.0
    });
    refs
}

/// A reference applied to an estimate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Comparison {
    pub key: String,
    pub quantity: Quantity,
    /// How many of the reference thing the estimate equals.
    pub count: f64,
    /// The same, at the low and high ends of the estimate's uncertainty.
    pub low: f64,
    pub high: f64,
    pub reference_value: f64,
    pub singular: String,
    pub plural: String,
    pub source: String,
}

impl Comparison {
    /// `3.1 days of a US household's electricity (2.5–3.7)`.
    pub fn phrase(&self) -> String {
        let noun = if rounds_to_one(self.count) {
            &self.singular
        } else {
            &self.plural
        };
        if (self.high - self.low).abs() > f64::EPSILON {
            format!(
                "{} {} ({}\u{2013}{})",
                fmt_count(self.count),
                noun,
                fmt_count(self.low),
                fmt_count(self.high)
            )
        } else {
            format!("{} {}", fmt_count(self.count), noun)
        }
    }
}

fn rounds_to_one(x: f64) -> bool {
    fmt_count(x) == "1"
}

/// Enough precision to read, no more: 0.23, 1.4, 12, 350.
pub fn fmt_count(x: f64) -> String {
    let s = if x >= 10.0 {
        format!("{:.0}", x)
    } else if x >= 1.0 {
        format!("{:.1}", x)
    } else {
        format!("{:.2}", x)
    };
    // Trim a trailing ".0" so 1.0 reads as 1.
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

/// Ratios inside this band read naturally; outside it a number is either
/// a fraction nobody pictures or a count nobody pictures.
const READABLE_LOW: f64 = 0.1;
const READABLE_HIGH: f64 = 1000.0;
/// Rank by closeness to this ratio: "about ten of something" is the most
/// legible size.
const IDEAL_RATIO_LOG10: f64 = 1.0;

/// Choose up to `take` comparisons for a value, best-scaled first.
/// If nothing lands in the readable band the nearest single reference is
/// returned, so a non-zero estimate always gets one comparison.
pub fn select(
    value: f64,
    low: f64,
    high: f64,
    quantity: Quantity,
    refs: &[Reference],
    take: usize,
) -> Vec<Comparison> {
    if value <= 0.0 {
        return Vec::new();
    }
    let mut scored: Vec<(f64, bool, Comparison)> = refs
        .iter()
        .filter(|r| r.quantity == quantity)
        .map(|r| {
            let ratio = value / r.value;
            let distance = (ratio.log10() - IDEAL_RATIO_LOG10).abs();
            let readable = (READABLE_LOW..=READABLE_HIGH).contains(&ratio);
            (
                distance,
                readable,
                Comparison {
                    key: r.key.clone(),
                    quantity,
                    count: ratio,
                    low: low / r.value,
                    high: high / r.value,
                    reference_value: r.value,
                    singular: r.singular.clone(),
                    plural: r.plural.clone(),
                    source: r.source.clone(),
                },
            )
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let readable: Vec<Comparison> = scored
        .iter()
        .filter(|(_, ok, _)| *ok)
        .map(|(_, _, c)| c.clone())
        .take(take)
        .collect();
    if !readable.is_empty() {
        return readable;
    }
    scored.into_iter().map(|(_, _, c)| c).take(1).collect()
}

/// Average daily rates over the interval the estimate covers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerDay {
    pub days: u32,
    pub wh: f64,
    pub co2_kg: Option<f64>,
    pub water_liters: Option<f64>,
}

/// Everything the report needs to say "equivalent to".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Comparisons {
    pub energy: Vec<Comparison>,
    pub co2: Vec<Comparison>,
    pub water: Vec<Comparison>,
    /// Caveats the output must carry alongside the numbers.
    pub notes: Vec<String>,
}

impl Comparisons {
    /// Build comparisons for an estimate. `co2_kg` and `water_liters` are
    /// scaled by the same relative uncertainty as the energy figure.
    pub fn for_estimate(
        total_wh: f64,
        uncertainty_range: (f64, f64),
        co2_kg: Option<f64>,
        water_liters: Option<f64>,
        config: &EnergyConfig,
    ) -> Self {
        let refs = references_with(&config.comparisons);
        let (low_wh, high_wh) = uncertainty_range;
        let (lo_f, hi_f) = if total_wh > 0.0 {
            (low_wh / total_wh, high_wh / total_wh)
        } else {
            (1.0, 1.0)
        };

        let energy = select(total_wh, low_wh, high_wh, Quantity::Energy, &refs, 2);
        let co2 = co2_kg
            .map(|c| select(c, c * lo_f, c * hi_f, Quantity::Co2, &refs, 2))
            .unwrap_or_default();
        let water = water_liters
            .map(|w| select(w, w * lo_f, w * hi_f, Quantity::Water, &refs, 2))
            .unwrap_or_default();

        let mut notes = vec![
            "Comparisons are illustrative, not measurements; each carries the energy estimate's uncertainty range.".to_string(),
        ];
        if let Some(gi) = config.grid_carbon_intensity {
            notes.push(format!(
                "CO2 uses a location-based grid intensity of {:.2} kg/kWh. Providers report market-based figures using renewable purchase agreements that can be near zero; both views are defensible.",
                gi
            ));
        }
        if config.water_liters_per_kwh.is_some() {
            notes.push(
                "Water covers on-site cooling plus off-site electricity generation (the scope of Li et al. 2023).".to_string(),
            );
        }

        Self {
            energy,
            co2,
            water,
            notes,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.energy.is_empty() && self.co2.is_empty() && self.water.is_empty()
    }
}

/// Daily averages for a total over `days` (never divides by zero).
pub fn per_day(days: u32, total_wh: f64, co2_kg: Option<f64>, water_liters: Option<f64>) -> PerDay {
    let d = days.max(1) as f64;
    PerDay {
        days: days.max(1),
        wh: total_wh / d,
        co2_kg: co2_kg.map(|c| c / d),
        water_liters: water_liters.map(|w| w / d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_count_reads_naturally() {
        assert_eq!(fmt_count(0.234), "0.23");
        assert_eq!(fmt_count(1.0), "1");
        assert_eq!(fmt_count(1.44), "1.4");
        assert_eq!(fmt_count(12.6), "13");
        assert_eq!(fmt_count(350.0), "350");
    }

    #[test]
    fn singular_only_when_it_rounds_to_one() {
        let c = |count: f64| Comparison {
            key: "k".into(),
            quantity: Quantity::Water,
            count,
            low: count,
            high: count,
            reference_value: 1.0,
            singular: "shower".into(),
            plural: "showers".into(),
            source: "".into(),
        };
        assert_eq!(c(1.0).phrase(), "1 shower");
        assert_eq!(c(1.04).phrase(), "1 shower");
        assert_eq!(c(1.4).phrase(), "1.4 showers");
        assert_eq!(c(0.5).phrase(), "0.50 showers");
    }
}
