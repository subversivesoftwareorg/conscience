//! Longitudinal view over a project's saved snapshots.
//!
//! Every `examine` writes a snapshot; this reads them back and compares
//! like with like. Snapshots are grouped by the interval they cover, and
//! each group's latest is compared to its previous one. Pull request
//! snapshots are listed but never compared: a PR's lifetime is not a
//! window. Nothing is ever summed across snapshots.

use crate::ethics::models::Severity;
use crate::snapshot::{Metric, MetricKind, Snapshot};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The headline metrics history tracks, in display order.
pub const TRACKED: &[&str] = &[
    "ai.sessions",
    "ai.turn_ratio",
    "ai.output_tokens",
    "energy.wh",
    "github.prs_merged",
    "github.review_comments_per_merged_pr",
];

/// One snapshot, reduced to what history needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub snapshot_id: String,
    pub collected_at: DateTime<Utc>,
    pub interval_start: DateTime<Utc>,
    pub interval_end: DateTime<Utc>,
    /// Whole days the interval spans; the grouping key.
    pub interval_days: u32,
    /// True when the interval was a pull request's lifetime.
    pub is_pr: bool,
    pub coverage: String,
    pub warnings: usize,
    pub concerns: usize,
    pub signal_ids: Vec<(String, String, Severity)>,
    pub metrics: BTreeMap<String, Metric>,
    pub config_fingerprint: String,
    pub analyzer_version: String,
}

impl Entry {
    pub fn from_snapshot(s: &Snapshot) -> Self {
        let is_pr = s
            .coverage
            .source("github")
            .map(|g| g.detail.starts_with("PR with"))
            .unwrap_or(false);
        Self {
            snapshot_id: s.snapshot_id.clone(),
            collected_at: s.interval.collected_at,
            interval_start: s.interval.start,
            interval_end: s.interval.end,
            interval_days: s.interval.days(),
            is_pr,
            coverage: s.coverage.summary(),
            warnings: s.signal_count(Severity::Warning),
            concerns: s.signal_count(Severity::Concern),
            signal_ids: s
                .analysis
                .signals
                .iter()
                .map(|x| (x.id.clone(), x.title.clone(), x.severity))
                .collect(),
            metrics: s
                .metrics
                .iter()
                .map(|m| (m.key.clone(), m.clone()))
                .collect(),
            config_fingerprint: s.analyzer.config_fingerprint.clone(),
            analyzer_version: s.analyzer.version.clone(),
        }
    }

    pub fn metric(&self, key: &str) -> Option<f64> {
        self.metrics.get(key).map(|m| m.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricChange {
    pub key: String,
    pub unit: String,
    pub kind: MetricKind,
    pub from: f64,
    pub to: f64,
    pub delta: f64,
    /// Percent change; `None` when `from` is zero.
    pub pct: Option<f64>,
}

/// Latest snapshot of a group against the previous one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Comparison {
    pub interval_days: u32,
    pub from_id: String,
    pub to_id: String,
    pub from_collected: DateTime<Utc>,
    pub to_collected: DateTime<Utc>,
    pub metrics: Vec<MetricChange>,
    /// Signals present now that were not before: (id, title, severity).
    pub appeared: Vec<(String, String, Severity)>,
    /// Signals present before that are gone now.
    pub resolved: Vec<(String, String, Severity)>,
    pub warnings: (usize, usize),
    pub concerns: (usize, usize),
    /// The thresholds changed between the two, so differences may be config.
    pub thresholds_changed: bool,
    pub version_changed: bool,
    /// Plain-language answers to the questions history exists to answer.
    pub readings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub entries: Vec<Entry>,
    pub comparisons: Vec<Comparison>,
    /// Groups with only one snapshot: nothing to compare yet.
    pub singletons: Vec<u32>,
}

/// Snapshots collected within the window, newest first.
pub fn load(root: &std::path::Path, since: DateTime<Utc>) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Snapshot::list(root)
        .iter()
        .filter_map(|p| Snapshot::load(p).ok())
        .filter(|s| s.interval.collected_at >= since)
        .map(|s| Entry::from_snapshot(&s))
        .collect();
    entries.sort_by(|a, b| b.collected_at.cmp(&a.collected_at));
    entries
}

/// Build the history: group comparable entries and compare each group's
/// latest two.
pub fn build(entries: Vec<Entry>) -> History {
    let mut groups: BTreeMap<u32, Vec<&Entry>> = BTreeMap::new();
    for e in entries.iter().filter(|e| !e.is_pr) {
        groups.entry(e.interval_days).or_default().push(e);
    }

    let mut comparisons = Vec::new();
    let mut singletons = Vec::new();
    for (days, group) in &groups {
        // Entries are newest first.
        match (group.first(), group.get(1)) {
            (Some(latest), Some(previous)) => comparisons.push(compare(previous, latest)),
            _ => singletons.push(*days),
        }
    }

    History {
        entries,
        comparisons,
        singletons,
    }
}

pub fn compare(from: &Entry, to: &Entry) -> Comparison {
    let mut metrics = Vec::new();
    for key in TRACKED {
        if let (Some(a), Some(b)) = (from.metrics.get(*key), to.metrics.get(*key)) {
            let delta = b.value - a.value;
            let pct = if a.value.abs() > f64::EPSILON {
                Some(delta / a.value * 100.0)
            } else {
                None
            };
            metrics.push(MetricChange {
                key: key.to_string(),
                unit: b.unit.clone(),
                kind: b.kind,
                from: a.value,
                to: b.value,
                delta,
                pct,
            });
        }
    }

    let from_ids: std::collections::HashSet<&str> = from
        .signal_ids
        .iter()
        .map(|(id, _, _)| id.as_str())
        .collect();
    let to_ids: std::collections::HashSet<&str> =
        to.signal_ids.iter().map(|(id, _, _)| id.as_str()).collect();
    let appeared: Vec<_> = to
        .signal_ids
        .iter()
        .filter(|(id, _, _)| !from_ids.contains(id.as_str()))
        .cloned()
        .collect();
    let resolved: Vec<_> = from
        .signal_ids
        .iter()
        .filter(|(id, _, _)| !to_ids.contains(id.as_str()))
        .cloned()
        .collect();

    let thresholds_changed = from.config_fingerprint != to.config_fingerprint;
    let version_changed = from.analyzer_version != to.analyzer_version;

    let readings = readings(from, to, &metrics, thresholds_changed);

    Comparison {
        interval_days: to.interval_days,
        from_id: from.snapshot_id.clone(),
        to_id: to.snapshot_id.clone(),
        from_collected: from.collected_at,
        to_collected: to.collected_at,
        metrics,
        appeared,
        resolved,
        warnings: (from.warnings, to.warnings),
        concerns: (from.concerns, to.concerns),
        thresholds_changed,
        version_changed,
        readings,
    }
}

/// The four questions from the roadmap issue, answered from the data when
/// it allows and stated as unknown when it doesn't.
fn readings(
    from: &Entry,
    to: &Entry,
    metrics: &[MetricChange],
    thresholds_changed: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    let change = |key: &str| metrics.iter().find(|m| m.key == key);
    let direction = |pct: f64| -> &'static str {
        if pct > 10.0 {
            "rising"
        } else if pct < -10.0 {
            "falling"
        } else {
            "roughly stable"
        }
    };

    match change("ai.turn_ratio").and_then(|m| m.pct.map(|p| (m, p))) {
        Some((m, p)) => out.push(format!(
            "AI:human turn ratio is {} ({:.1} to {:.1}).",
            direction(p),
            m.from,
            m.to
        )),
        None => out.push("AI:human turn ratio: not enough data to compare.".into()),
    }

    match change("github.review_comments_per_merged_pr").and_then(|m| m.pct.map(|p| (m, p))) {
        Some((m, p)) => out.push(format!(
            "Review comments per merged PR are {} ({:.1} to {:.1}).",
            direction(p),
            m.from,
            m.to
        )),
        None => out.push("Review engagement: no merged PRs in both periods to compare.".into()),
    }

    // Energy relative to output: is compute growing faster than what it produces?
    let per_token = |e: &Entry| match (e.metric("energy.wh"), e.metric("ai.output_tokens")) {
        (Some(wh), Some(tok)) if tok > 0.0 => Some(wh / tok * 1000.0),
        _ => None,
    };
    match (per_token(from), per_token(to)) {
        (Some(a), Some(b)) if a > 0.0 => {
            let p = (b - a) / a * 100.0;
            out.push(format!(
                "Energy per 1K output tokens is {} (~{:.2} to ~{:.2} Wh, estimated).",
                direction(p),
                a,
                b
            ));
        }
        _ => out.push("Energy proportionality: not enough data to compare.".into()),
    }

    let security = |e: &Entry| {
        e.signal_ids
            .iter()
            .filter(|(id, _, sev)| id.starts_with("security_") && *sev >= Severity::Concern)
            .count()
    };
    let (sa, sb) = (security(from), security(to));
    out.push(match sb.cmp(&sa) {
        std::cmp::Ordering::Less => {
            format!("Security signals are trending down ({} to {}).", sa, sb)
        }
        std::cmp::Ordering::Greater => {
            format!("Security signals are trending up ({} to {}).", sa, sb)
        }
        std::cmp::Ordering::Equal if sa == 0 => "No security signals in either period.".into(),
        std::cmp::Ordering::Equal => format!("Security signals are unchanged ({}).", sa),
    });

    if thresholds_changed {
        out.push(
            "Thresholds changed between these snapshots; some differences may be configuration rather than behaviour."
                .into(),
        );
    }
    out
}

/// Counts read as integers (`2`, not `2.00`); ratios keep a decimal or two.
fn fmt_value(v: f64, unit: &str) -> String {
    if unit == "tokens" && v >= 1_000.0 {
        format!("{:.0}K", v / 1_000.0)
    } else if v.fract().abs() < 1e-9 || v >= 100.0 {
        format!("{:.0}", v)
    } else if v >= 10.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_value;

    #[test]
    fn counts_are_integers_and_ratios_keep_decimals() {
        assert_eq!(fmt_value(2.0, "sessions"), "2");
        assert_eq!(fmt_value(3.0, "prs"), "3");
        assert_eq!(fmt_value(0.0, "comments"), "0");
        assert_eq!(fmt_value(20.04, "assistant_turns"), "20.0");
        assert_eq!(fmt_value(1.5, "comments"), "1.50");
        assert_eq!(fmt_value(2_653_000.0, "tokens"), "2653K");
        assert_eq!(fmt_value(28_789.4, "Wh"), "28789");
    }
}

/// Text rendering for the terminal.
pub fn render(h: &History) -> String {
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  Conscience \u{2014} History");
    let _ = writeln!(out);

    if h.entries.is_empty() {
        let _ = writeln!(
            out,
            "  No snapshots in the window. Run `conscience examine` to create one."
        );
        return out;
    }

    let _ = writeln!(out, "  Snapshots ({}, newest first)", h.entries.len());
    let _ = writeln!(out);
    for e in &h.entries {
        let kind = if e.is_pr {
            "PR".to_string()
        } else {
            format!("{}d", e.interval_days)
        };
        let mut cols = vec![
            format!("{:<4}", kind),
            format!("{} W/{} C", e.warnings, e.concerns),
        ];
        for key in TRACKED {
            if let Some(m) = e.metrics.get(*key) {
                let mark = if m.kind == MetricKind::Estimate {
                    "~"
                } else {
                    ""
                };
                let short = key.rsplit('.').next().unwrap_or(key);
                cols.push(format!("{} {}{}", short, mark, fmt_value(m.value, &m.unit)));
            }
        }
        let _ = writeln!(
            out,
            "  {}  {}  {}",
            e.collected_at.format("%Y-%m-%d %H:%M"),
            e.snapshot_id,
            cols.join(" | ")
        );
    }
    let _ = writeln!(out);

    if h.comparisons.is_empty() {
        let _ = writeln!(
            out,
            "  Nothing to compare yet: each interval length has one snapshot. Run `conscience examine` again later with the same --days."
        );
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  ~ marks estimates. Snapshots are separate assessments; nothing here is summed across them."
        );
        let _ = writeln!(out);
        return out;
    }

    for c in &h.comparisons {
        let _ = writeln!(
            out,
            "  Change over the {}-day window: {} ({}) \u{2192} {} ({})",
            c.interval_days,
            c.from_id,
            c.from_collected.format("%Y-%m-%d"),
            c.to_id,
            c.to_collected.format("%Y-%m-%d")
        );
        let _ = writeln!(out);
        for m in &c.metrics {
            let mark = if m.kind == MetricKind::Estimate {
                "~"
            } else {
                ""
            };
            let pct = m
                .pct
                .map(|p| format!("{:+.0}%", p))
                .unwrap_or_else(|| "n/a".into());
            let _ = writeln!(
                out,
                "    {:<38} {}{} \u{2192} {}{}  ({})",
                m.key,
                mark,
                fmt_value(m.from, &m.unit),
                mark,
                fmt_value(m.to, &m.unit),
                pct
            );
        }
        let _ = writeln!(
            out,
            "    {:<38} {} \u{2192} {}",
            "warnings / concerns",
            format!("{} / {}", c.warnings.0, c.concerns.0),
            format!("{} / {}", c.warnings.1, c.concerns.1)
        );
        if !c.appeared.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "    Appeared:");
            for (_, title, sev) in &c.appeared {
                let _ = writeln!(out, "      {:>7} {}", sev.to_string(), title);
            }
        }
        if !c.resolved.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "    Resolved:");
            for (_, title, sev) in &c.resolved {
                let _ = writeln!(out, "      {:>7} {}", sev.to_string(), title);
            }
        }
        if c.version_changed {
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "    Note: analyzer version changed between these snapshots."
            );
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "    Readings");
        for r in &c.readings {
            let _ = writeln!(out, "      - {}", r);
        }
        let _ = writeln!(out);
    }

    if !h.singletons.is_empty() {
        let _ = writeln!(
            out,
            "  Only one snapshot for: {}. Nothing to compare there yet.",
            h.singletons
                .iter()
                .map(|d| format!("{}-day", d))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let _ = writeln!(out);
    }

    let _ = writeln!(
        out,
        "  ~ marks estimates. Snapshots are separate assessments; nothing here is summed across them."
    );
    let _ = writeln!(out);
    out
}
