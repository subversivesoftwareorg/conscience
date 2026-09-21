//! What conscience itself takes up on disk, and how to tidy it.
//!
//! Two kinds of paths, kept firmly apart. **Owned**: things conscience
//! wrote and may remove: snapshots, saved reflections, its config, and the
//! backups `prune` keeps. **Read**: the AI tool logs it analyzes, which it
//! never writes and never deletes. Showing both answers the real question:
//! conscience's own cost is small; the thing that grows is what it reads.

use crate::config::Config;
use crate::error::{ConscienceError, Result};
use crate::snapshot::Snapshot;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnedEntry {
    pub path: String,
    /// `snapshots`, `reflections`, `config`, or `prune backups`.
    pub kind: String,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadEntry {
    pub path: String,
    pub tool: String,
    pub files: u64,
    pub bytes: u64,
}

/// How fast snapshots are accumulating for one project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Growth {
    pub project: String,
    pub snapshots_last_30_days: u64,
    pub bytes_last_30_days: u64,
    pub bytes_per_year_at_this_rate: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub owned: Vec<OwnedEntry>,
    pub owned_bytes: u64,
    pub read: Vec<ReadEntry>,
    pub read_bytes: u64,
    pub growth: Vec<Growth>,
}

fn dir_size(path: &Path) -> (u64, u64) {
    let mut files = 0u64;
    let mut bytes = 0u64;
    if path.is_file() {
        return (1, path.metadata().map(|m| m.len()).unwrap_or(0));
    }
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(m) = p.metadata() {
                files += 1;
                bytes += m.len();
            }
        }
    }
    (files, bytes)
}

/// Measure conscience's footprint for the given project roots plus the
/// global locations under `home`.
pub fn measure(project_roots: &[PathBuf], home: &Path) -> Usage {
    let mut usage = Usage::default();

    for root in project_roots {
        let base = root.join(".conscience");
        for (sub, kind) in [("snapshots", "snapshots"), ("reflections", "reflections")] {
            let p = base.join(sub);
            if p.is_dir() {
                let (files, bytes) = dir_size(&p);
                if files > 0 {
                    usage.owned.push(OwnedEntry {
                        path: p.to_string_lossy().to_string(),
                        kind: kind.into(),
                        files,
                        bytes,
                    });
                }
            }
        }
        if let Some(g) = growth_for(root) {
            usage.growth.push(g);
        }
    }

    let config = Config::config_path();
    if config.is_file() {
        let (files, bytes) = dir_size(&config);
        usage.owned.push(OwnedEntry {
            path: config.to_string_lossy().to_string(),
            kind: "config".into(),
            files,
            bytes,
        });
    }
    let pruned = home.join(".claude").join("pruned");
    if pruned.is_dir() {
        let (files, bytes) = dir_size(&pruned);
        if files > 0 {
            usage.owned.push(OwnedEntry {
                path: pruned.to_string_lossy().to_string(),
                kind: "prune backups".into(),
                files,
                bytes,
            });
        }
    }
    usage.owned_bytes = usage.owned.iter().map(|e| e.bytes).sum();

    for (rel, tool) in [
        (".claude/projects", "Claude Code"),
        (".codex/sessions", "Codex"),
    ] {
        let p = home.join(rel);
        if p.is_dir() {
            let (files, bytes) = dir_size(&p);
            usage.read.push(ReadEntry {
                path: p.to_string_lossy().to_string(),
                tool: tool.into(),
                files,
                bytes,
            });
        }
    }
    usage.read_bytes = usage.read.iter().map(|e| e.bytes).sum();

    usage
}

fn growth_for(root: &Path) -> Option<Growth> {
    let since = Utc::now() - Duration::days(30);
    let recent: Vec<PathBuf> = Snapshot::list(root)
        .into_iter()
        .filter(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .map(|t| DateTime::<Utc>::from(t) >= since)
                .unwrap_or(false)
        })
        .collect();
    if recent.is_empty() {
        return None;
    }
    let bytes: u64 = recent
        .iter()
        .filter_map(|p| p.metadata().ok().map(|m| m.len()))
        .sum();
    Some(Growth {
        project: root.to_string_lossy().to_string(),
        snapshots_last_30_days: recent.len() as u64,
        bytes_last_30_days: bytes,
        bytes_per_year_at_this_rate: bytes * 365 / 30,
    })
}

pub fn fmt_bytes(b: u64) -> String {
    const K: f64 = 1024.0;
    let f = b as f64;
    if f < K {
        format!("{} B", b)
    } else if f < K * K {
        format!("{:.0} KB", f / K)
    } else if f < K * K * K {
        format!("{:.1} MB", f / K / K)
    } else {
        format!("{:.2} GB", f / K / K / K)
    }
}

pub fn render(u: &Usage) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  Conscience \u{2014} Disk Usage");
    let _ = writeln!(out);
    let _ = writeln!(out, "  Owned by conscience (may be tidied)");
    if u.owned.is_empty() {
        let _ = writeln!(out, "    nothing written yet");
    }
    for e in &u.owned {
        let _ = writeln!(
            out,
            "    {:>9}  {:>4} file(s)  {:<15} {}",
            fmt_bytes(e.bytes),
            e.files,
            e.kind,
            e.path
        );
    }
    let _ = writeln!(out, "    {:>9}  total", fmt_bytes(u.owned_bytes));
    let _ = writeln!(out);
    let _ = writeln!(out, "  Read, never written or deleted");
    if u.read.is_empty() {
        let _ = writeln!(out, "    no AI tool logs found");
    }
    for e in &u.read {
        let _ = writeln!(
            out,
            "    {:>9}  {:>4} file(s)  {:<15} {}",
            fmt_bytes(e.bytes),
            e.files,
            e.tool,
            e.path
        );
    }
    let _ = writeln!(out, "    {:>9}  total", fmt_bytes(u.read_bytes));
    let _ = writeln!(out);
    if u.read_bytes > 0 && u.owned_bytes > 0 {
        let ratio = u.read_bytes as f64 / u.owned_bytes as f64;
        let _ = writeln!(
            out,
            "  Conscience holds {} and reads {}: about {:.0}x more is read than written.",
            fmt_bytes(u.owned_bytes),
            fmt_bytes(u.read_bytes),
            ratio
        );
    }
    for g in &u.growth {
        let _ = writeln!(
            out,
            "  {}: {} snapshot(s) in the last 30 days ({}); ~{} per year at this rate.",
            g.project,
            g.snapshots_last_30_days,
            fmt_bytes(g.bytes_last_30_days),
            fmt_bytes(g.bytes_per_year_at_this_rate)
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  Snapshots are the only thing that accumulates. `conscience du --tidy` removes old ones while keeping what history compares."
    );
    let _ = writeln!(out);
    out
}

/// What `--tidy` would remove: snapshots older than the window, except the
/// two most recent of each interval length (and of PR snapshots), which
/// history needs. Reflections are never touched.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TidyPlan {
    pub project: String,
    pub remove: Vec<(String, u64)>,
    pub kept: u64,
    pub bytes_freed: u64,
}

pub fn plan_tidy(root: &Path, keep_days: u32) -> TidyPlan {
    let cutoff = Utc::now() - Duration::days(keep_days as i64);
    let mut plan = TidyPlan {
        project: root.to_string_lossy().to_string(),
        ..Default::default()
    };

    // Snapshot::list is newest first (ids sort by time).
    let mut per_group: BTreeMap<String, u64> = BTreeMap::new();
    for path in Snapshot::list(root) {
        let Ok(snap) = Snapshot::load(&path) else {
            continue;
        };
        let is_pr = snap
            .coverage
            .source("github")
            .map(|g| g.detail.starts_with("PR with"))
            .unwrap_or(false);
        let group = if is_pr {
            "pr".to_string()
        } else {
            format!("{}d", snap.interval.days())
        };
        let seen = per_group.entry(group).or_default();
        *seen += 1;
        let protected = *seen <= 2;
        let old = snap.interval.collected_at < cutoff;
        if old && !protected {
            let bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
            plan.remove
                .push((path.to_string_lossy().to_string(), bytes));
            plan.bytes_freed += bytes;
        } else {
            plan.kept += 1;
        }
    }
    plan
}

pub fn apply_tidy(plan: &TidyPlan) -> Result<u64> {
    let mut removed = 0u64;
    for (path, _) in &plan.remove {
        std::fs::remove_file(path).map_err(|e| {
            ConscienceError::Other(anyhow::anyhow!("cannot remove {}: {}", path, e))
        })?;
        removed += 1;
    }
    Ok(removed)
}

pub fn render_tidy(plan: &TidyPlan, keep_days: u32) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  Tidy plan for {}", plan.project);
    let _ = writeln!(
        out,
        "  Remove snapshots older than {} days, keeping the two most recent of each interval length: {} to remove ({}), {} kept.",
        keep_days,
        plan.remove.len(),
        fmt_bytes(plan.bytes_freed),
        plan.kept
    );
    for (path, bytes) in &plan.remove {
        let _ = writeln!(out, "    {:>9}  {}", fmt_bytes(*bytes), path);
    }
    let _ = writeln!(out, "  Reflections and AI tool logs are not touched.");
    let _ = writeln!(out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_read_naturally() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(900), "900 B");
        assert_eq!(fmt_bytes(20 * 1024), "20 KB");
        assert_eq!(fmt_bytes(356 * 1024 * 1024), "356.0 MB");
        assert_eq!(fmt_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}
