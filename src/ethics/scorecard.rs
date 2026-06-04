use crate::ethics::models::*;

/// Build a scorecard from detected signals.
/// Groups signals by principle and marks which dimensions need human input.
pub fn build_scorecard(signals: &[Signal]) -> Vec<DimensionScore> {
    Principle::all()
        .iter()
        .map(|principle| {
            let auto_signals: Vec<Signal> = signals
                .iter()
                .filter(|s| s.principle == *principle)
                .cloned()
                .collect();

            let needs_human = match principle {
                Principle::CodeProvenance => true,
                Principle::Security if auto_signals.is_empty() => true,
                Principle::DeveloperGrowth if auto_signals.is_empty() => true,
                Principle::EquityOfBenefit if auto_signals.is_empty() => true,
                _ => false,
            };

            DimensionScore {
                principle: *principle,
                auto_signals,
                needs_human_input: needs_human,
                human_assessment: None,
            }
        })
        .collect()
}
