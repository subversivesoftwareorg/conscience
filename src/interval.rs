//! One explicit time interval per command.
//!
//! Every command resolves its interval once, before any collection, and
//! passes the same value to GitHub and AI ingest. That keeps "the last
//! 30 days" meaning the same thing for PRs and for AI sessions, instead of
//! a 30-day PR window sitting next to lifetime session totals.

use crate::ai_tools::models::AiSession;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Interval {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// When the interval was resolved; also the moment data was collected.
    pub collected_at: DateTime<Utc>,
}

impl Interval {
    /// The last `days` days, ending now.
    pub fn last_days(days: u32) -> Self {
        let now = Utc::now();
        Self {
            start: now - Duration::days(days as i64),
            end: now,
            collected_at: now,
        }
    }

    /// The last `hours` hours, ending now.
    pub fn last_hours(hours: u32) -> Self {
        let now = Utc::now();
        Self {
            start: now - Duration::hours(hours as i64),
            end: now,
            collected_at: now,
        }
    }

    /// A relative window ending now, from a short spec: `90m`, `4h`, `2d`,
    /// `1w`. A bare number means hours.
    pub fn since(spec: &str) -> std::result::Result<Self, String> {
        let s = spec.trim();
        let (num, unit) = match s.find(|c: char| !c.is_ascii_digit()) {
            Some(i) => (&s[..i], s[i..].trim()),
            None => (s, "h"),
        };
        let n: i64 = num
            .parse()
            .map_err(|_| format!("invalid duration '{}': expected e.g. 90m, 4h, 2d, 1w", spec))?;
        if n <= 0 {
            return Err(format!("invalid duration '{}': must be positive", spec));
        }
        let span = match unit {
            "m" | "min" | "mins" | "minute" | "minutes" => Duration::minutes(n),
            "h" | "hr" | "hrs" | "hour" | "hours" => Duration::hours(n),
            "d" | "day" | "days" => Duration::days(n),
            "w" | "wk" | "week" | "weeks" => Duration::weeks(n),
            other => {
                return Err(format!(
                    "invalid duration unit '{}' in '{}': use m, h, d, or w",
                    other, spec
                ));
            }
        };
        let now = Utc::now();
        Ok(Self {
            start: now - span,
            end: now,
            collected_at: now,
        })
    }

    /// An explicit range, e.g. a pull request's lifetime.
    pub fn between(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            start,
            end,
            collected_at: Utc::now(),
        }
    }

    /// Whole days spanned, rounded up, never less than one.
    pub fn days(&self) -> u32 {
        let secs = (self.end - self.start).num_seconds().max(0);
        ((secs + 86_399) / 86_400).max(1) as u32
    }

    /// Does an instant fall inside the interval (inclusive)?
    pub fn contains(&self, at: DateTime<Utc>) -> bool {
        at >= self.start && at <= self.end
    }

    /// Whether a session overlaps this interval.
    ///
    /// `None` means the session carries no timestamp at all, so the question
    /// cannot be answered; callers should count those separately rather than
    /// silently include or drop them.
    pub fn covers_session(&self, session: &AiSession) -> Option<bool> {
        let start = session.started_at.or(session.ended_at)?;
        let end = session.ended_at.or(session.started_at)?;
        Some(start <= self.end && end >= self.start)
    }

    /// Short human label, e.g. `2026-08-20 to 2026-09-19 (30 days)`.
    pub fn label(&self) -> String {
        if self.end - self.start < Duration::days(2) {
            format!(
                "{} to {} ({} hours)",
                self.start.format("%Y-%m-%d %H:%M"),
                self.end.format("%Y-%m-%d %H:%M"),
                (self.end - self.start).num_hours().max(1)
            )
        } else {
            format!(
                "{} to {} ({} days)",
                self.start.format("%Y-%m-%d"),
                self.end.format("%Y-%m-%d"),
                self.days()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_tools::models::*;
    use std::collections::HashMap;

    fn session(start: Option<&str>, end: Option<&str>) -> AiSession {
        let p = |s: &str| s.parse::<DateTime<Utc>>().unwrap();
        AiSession {
            tool: AiTool::ClaudeCode,
            session_id: "s".into(),
            project_path: None,
            started_at: start.map(p),
            ended_at: end.map(p),
            model: None,
            work_categories: vec![],
            turns: TurnCounts::default(),
            tokens: TokenUsage::default(),
            tools_used: HashMap::new(),
            files_touched: vec![],
            bash_commands: vec![],
            agent_actions: vec![],
            git_branch: None,
            interactions: vec![],
            agent_dispatches: vec![],
            skill_invocations: vec![],
        }
    }

    fn iv() -> Interval {
        Interval::between(
            "2026-09-01T00:00:00Z".parse().unwrap(),
            "2026-09-08T00:00:00Z".parse().unwrap(),
        )
    }

    #[test]
    fn session_inside_interval_is_covered() {
        let s = session(Some("2026-09-03T10:00:00Z"), Some("2026-09-03T11:00:00Z"));
        assert_eq!(iv().covers_session(&s), Some(true));
    }

    #[test]
    fn session_straddling_the_start_is_covered() {
        let s = session(Some("2026-08-31T22:00:00Z"), Some("2026-09-01T02:00:00Z"));
        assert_eq!(iv().covers_session(&s), Some(true));
    }

    #[test]
    fn session_entirely_before_or_after_is_not_covered() {
        let before = session(Some("2026-08-20T10:00:00Z"), Some("2026-08-20T11:00:00Z"));
        let after = session(Some("2026-09-09T10:00:00Z"), Some("2026-09-09T11:00:00Z"));
        assert_eq!(iv().covers_session(&before), Some(false));
        assert_eq!(iv().covers_session(&after), Some(false));
    }

    #[test]
    fn session_with_only_one_timestamp_uses_it() {
        let s = session(Some("2026-09-03T10:00:00Z"), None);
        assert_eq!(iv().covers_session(&s), Some(true));
        let s = session(None, Some("2026-08-03T10:00:00Z"));
        assert_eq!(iv().covers_session(&s), Some(false));
    }

    #[test]
    fn undated_session_is_neither_in_nor_out() {
        assert_eq!(iv().covers_session(&session(None, None)), None);
    }

    #[test]
    fn since_parses_common_specs_and_rejects_bad_ones() {
        let secs = |s: &str| {
            let iv = Interval::since(s).unwrap();
            (iv.end - iv.start).num_seconds()
        };
        assert_eq!(secs("90m"), 90 * 60);
        assert_eq!(secs("4h"), 4 * 3600);
        assert_eq!(secs("4"), 4 * 3600, "bare number is hours");
        assert_eq!(secs("2d"), 2 * 86_400);
        assert_eq!(secs("1w"), 7 * 86_400);
        assert_eq!(secs(" 3 hours "), 3 * 3600);
        assert!(Interval::since("0h").is_err());
        assert!(Interval::since("h").is_err());
        assert!(Interval::since("4x").is_err());
        assert!(Interval::since("").is_err());
    }

    #[test]
    fn days_rounds_up_and_never_below_one() {
        assert_eq!(iv().days(), 7);
        let short = Interval::between(
            "2026-09-01T00:00:00Z".parse().unwrap(),
            "2026-09-01T01:00:00Z".parse().unwrap(),
        );
        assert_eq!(short.days(), 1);
        assert!(short.label().contains("hours"));
        assert!(iv().label().contains("7 days"));
    }
}
