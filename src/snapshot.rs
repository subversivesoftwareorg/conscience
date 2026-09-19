//! Versioned analysis snapshot.
//!
//! A snapshot is one assessment of one project over one interval: what was
//! collected, how much of it was available, the numbers that came out with
//! their units and uncertainty, which analyzer and thresholds produced them,
//! and the signals and reflection questions themselves.
//!
//! Snapshots are what `examine` writes, what `push` uploads, and what
//! history compares. Two snapshots with overlapping intervals are separate
//! assessments; their totals must never be summed.

use crate::error::{ConscienceError, Result};
use crate::ethics::models::{EthicalAnalysis, Severity};
use crate::interval::Interval;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    /// `YYYYMMDDTHHMMSSZ-xxxxxx`; sortable by collection time.
    pub snapshot_id: String,
    pub project: ProjectIdentity,
    pub interval: Interval,
    pub coverage: Coverage,
    pub metrics: Vec<Metric>,
    pub analyzer: AnalyzerInfo,
    /// Flattened so `signals`, `scorecard`, and `reflections` stay top-level
    /// keys: existing consumers of `examine --json` keep working.
    #[serde(flatten)]
    pub analysis: EthicalAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIdentity {
    /// Hash of the canonical root path: stable on one machine. Use
    /// `github_repo` for identity shared across a team.
    pub id: String,
    pub name: String,
    pub root: String,
    pub github_repo: Option<String>,
    #[serde(default)]
    pub worktrees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Coverage {
    pub sources: Vec<SourceCoverage>,
    pub ai_sessions_in_range: u64,
    pub ai_sessions_undated: u64,
    pub github_commits: u64,
    pub github_pull_requests: u64,
    pub manifest_found: bool,
}

impl Coverage {
    pub fn source(&self, name: &str) -> Option<&SourceCoverage> {
        self.sources.iter().find(|s| s.source == name)
    }

    /// One line, e.g. `github collected (12 PRs); claude_code collected (26 sessions, 1 undated)`.
    pub fn summary(&self) -> String {
        self.sources
            .iter()
            .map(|s| {
                if s.detail.is_empty() {
                    format!("{} {}", s.source, s.status)
                } else {
                    format!("{} {} ({})", s.source, s.status, s.detail)
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCoverage {
    pub source: String,
    pub status: SourceStatus,
    /// Short human explanation: item counts, or why it was not collected.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    /// Data was fetched and is reflected in the metrics and signals.
    Collected,
    /// The source exists in principle but no data was found for this scope.
    Unavailable,
    /// Collection was attempted and errored; see `detail`.
    Failed,
    /// Not requested for this run (e.g. no `--repo`).
    Skipped,
}

impl std::fmt::Display for SourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SourceStatus::Collected => "collected",
            SourceStatus::Unavailable => "unavailable",
            SourceStatus::Failed => "failed",
            SourceStatus::Skipped => "skipped",
        };
        write!(f, "{}", s)
    }
}

/// One measured or estimated number, with enough context to compare it
/// honestly later: what it counts, per what, and how sure we are.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Dotted key, e.g. `ai.output_tokens`, `github.prs_merged`, `energy.wh`.
    pub key: String,
    pub value: f64,
    pub unit: String,
    /// What the value is per, when it is a ratio (e.g. `human_turns`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denominator: Option<String>,
    /// Low and high bounds when the value is an estimate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<(f64, f64)>,
    pub kind: MetricKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    /// Counted directly from the data.
    Observation,
    /// Derived through a model or coefficient; carries uncertainty.
    Estimate,
}

impl Metric {
    pub fn observed(key: &str, value: f64, unit: &str) -> Self {
        Self {
            key: key.into(),
            value,
            unit: unit.into(),
            denominator: None,
            uncertainty: None,
            kind: MetricKind::Observation,
        }
    }

    pub fn ratio(key: &str, value: f64, unit: &str, denominator: &str) -> Self {
        Self {
            denominator: Some(denominator.into()),
            ..Self::observed(key, value, unit)
        }
    }

    pub fn estimated(key: &str, value: f64, unit: &str, low: f64, high: f64) -> Self {
        Self {
            uncertainty: Some((low, high)),
            kind: MetricKind::Estimate,
            ..Self::observed(key, value, unit)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerInfo {
    /// Crate version that produced this snapshot.
    pub version: String,
    /// Fingerprint of the thresholds actually applied, so two snapshots can
    /// be told apart when only configuration changed.
    pub config_fingerprint: String,
    /// `manifest` when thresholds came from conscience.yaml, else `defaults`.
    pub thresholds_source: String,
}

impl Snapshot {
    pub fn signal_count(&self, severity: Severity) -> usize {
        self.analysis
            .signals
            .iter()
            .filter(|s| s.severity == severity)
            .count()
    }

    pub fn metric(&self, key: &str) -> Option<&Metric> {
        self.metrics.iter().find(|m| m.key == key)
    }

    /// Directory snapshots live in for a project root.
    pub fn dir_for(root: &Path) -> PathBuf {
        root.join(".conscience").join("snapshots")
    }

    /// Write to `<root>/.conscience/snapshots/<id>.json`.
    pub fn save(&self, root: &Path) -> Result<PathBuf> {
        let dir = Self::dir_for(root);
        std::fs::create_dir_all(&dir).map_err(|e| {
            ConscienceError::Other(anyhow::anyhow!("Cannot create {}: {}", dir.display(), e))
        })?;
        let path = dir.join(format!("{}.json", self.snapshot_id));
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            ConscienceError::Other(anyhow::anyhow!("Cannot serialize snapshot: {}", e))
        })?;
        std::fs::write(&path, json).map_err(|e| {
            ConscienceError::Other(anyhow::anyhow!("Cannot write {}: {}", path.display(), e))
        })?;
        Ok(path)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ConscienceError::Other(anyhow::anyhow!("Cannot read {}: {}", path.display(), e))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            ConscienceError::Other(anyhow::anyhow!(
                "{} is not a snapshot: {}",
                path.display(),
                e
            ))
        })
    }

    /// Every snapshot file for a project, newest first (IDs sort by time).
    pub fn list(root: &Path) -> Vec<PathBuf> {
        let dir = Self::dir_for(root);
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        paths.sort();
        paths.reverse();
        paths
    }

    /// The most recent snapshot for a project, if any.
    pub fn latest(root: &Path) -> Result<Option<(PathBuf, Self)>> {
        match Self::list(root).into_iter().next() {
            Some(p) => Ok(Some((p.clone(), Self::load(&p)?))),
            None => Ok(None),
        }
    }

    /// Resolve `id_or_path`: a file path, or a snapshot ID (or unique prefix)
    /// under the project's snapshot directory.
    pub fn find(root: &Path, id_or_path: &str) -> Result<(PathBuf, Self)> {
        let as_path = Path::new(id_or_path);
        if as_path.is_file() {
            return Ok((as_path.to_path_buf(), Self::load(as_path)?));
        }
        let matches: Vec<PathBuf> = Self::list(root)
            .into_iter()
            .filter(|p| {
                p.file_stem()
                    .map(|s| s.to_string_lossy().starts_with(id_or_path))
                    .unwrap_or(false)
            })
            .collect();
        match matches.len() {
            1 => Ok((matches[0].clone(), Self::load(&matches[0])?)),
            0 => Err(ConscienceError::Other(anyhow::anyhow!(
                "No snapshot matching '{}' under {}",
                id_or_path,
                Self::dir_for(root).display()
            ))),
            n => Err(ConscienceError::Other(anyhow::anyhow!(
                "'{}' matches {} snapshots; give more of the id",
                id_or_path,
                n
            ))),
        }
    }

    /// A few lines a person can read before deciding to upload.
    pub fn summary(&self) -> String {
        let warnings = self.signal_count(Severity::Warning);
        let concerns = self.signal_count(Severity::Concern);
        format!(
            "Snapshot {}\n  Project:  {} ({})\n  Interval: {}\n  Coverage: {}\n  Signals:  {} ({} warnings, {} concerns)\n  Analyzer: conscience {} / thresholds {} ({})",
            self.snapshot_id,
            self.project.name,
            self.project.root,
            self.interval.label(),
            self.coverage.summary(),
            self.analysis.signals.len(),
            warnings,
            concerns,
            self.analyzer.version,
            self.analyzer.config_fingerprint,
            self.analyzer.thresholds_source,
        )
    }
}

/// Timestamp to the second plus six hex characters of entropy from the
/// nanosecond clock, process ID, and an in-process counter. Shared by
/// snapshots and reflection sessions so both kinds of record sort by time
/// and never collide, without a UUID dependency for one identifier.
pub fn stamp_id(now: DateTime<Utc>) -> String {
    use std::hash::{Hash, Hasher};
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut h = std::collections::hash_map::DefaultHasher::new();
    now.timestamp_nanos_opt().unwrap_or_default().hash(&mut h);
    std::process::id().hash(&mut h);
    COUNTER
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .hash(&mut h);
    format!(
        "{}-{:06x}",
        now.format("%Y%m%dT%H%M%SZ"),
        h.finish() & 0xff_ffff
    )
}

/// FNV-1a over bytes. Stable across Rust versions and platforms, unlike the
/// standard library's default hasher, so fingerprints written today still
/// match ones computed next year.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Sixteen hex characters identifying any serializable value by content.
pub fn fingerprint<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("{:016x}", fnv1a(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_reference_vectors() {
        // Published FNV-1a 64-bit test vectors.
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x85944171f73967e8);
    }

    #[test]
    fn fingerprint_is_stable_and_content_sensitive() {
        #[derive(Serialize)]
        struct T {
            a: u32,
            b: &'static str,
        }
        let x = fingerprint(&T { a: 1, b: "x" });
        assert_eq!(x, fingerprint(&T { a: 1, b: "x" }));
        assert_ne!(x, fingerprint(&T { a: 2, b: "x" }));
        assert_eq!(x.len(), 16);
    }

    #[test]
    fn stamp_ids_are_unique_and_time_prefixed() {
        let now = Utc::now();
        let a = stamp_id(now);
        let b = stamp_id(now);
        assert_ne!(a, b);
        assert_eq!(&a[..16], &b[..16]);
        assert_eq!(a.len(), 23);
    }
}
