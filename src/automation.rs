//! Work that runs without a person present: cron entries and launchd
//! agents that invoke Claude, Claude Code daemon jobs, and sessions started
//! by `claude -p` or the SDK. `report automation` lists them with what they
//! have actually produced; `prune` removes the launcher, never the record.
//!
//! The observations here follow the project's rule: they say what the data
//! shows ("has never succeeded", "blocked awaiting input for 46 days") and
//! leave the judgment to the reader.

use crate::ai_tools::models::AiSession;
use crate::analysis::energy;
use crate::error::{ConscienceError, Result};
use crate::ethics::manifest::EnergyConfig;
use crate::snapshot::fnv1a;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where an automated job is defined.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    /// A line in the user's crontab.
    Cron {
        line: String,
        schedule: String,
        command: String,
    },
    /// A launchd agent plist that mentions claude.
    Launchd { path: String, label: String },
    /// A Claude Code daemon background job.
    DaemonJob {
        dir: String,
        name: String,
        state: String,
        needs: Option<String>,
        created_at: Option<DateTime<Utc>>,
    },
    /// Sessions launched by a program, with no launcher we could find.
    SdkSignature { cwd: String, prompt: String },
}

impl Source {
    pub fn kind(&self) -> &'static str {
        match self {
            Source::Cron { .. } => "cron",
            Source::Launchd { .. } => "launchd",
            Source::DaemonJob { .. } => "daemon job",
            Source::SdkSignature { .. } => "sdk runs",
        }
    }

    /// Stable identity: the same source yields the same id across runs.
    fn key(&self) -> String {
        match self {
            Source::Cron { line, .. } => format!("cron:{}", line.trim()),
            Source::Launchd { path, .. } => format!("launchd:{}", path),
            Source::DaemonJob { dir, .. } => format!("job:{}", dir),
            Source::SdkSignature { cwd, prompt } => format!("sdk:{}:{}", cwd, prompt),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    /// Six hex characters derived from the source; what `prune` takes.
    pub id: String,
    pub source: Source,
    /// What it is, in a few words: the prompt, the job name, the command.
    pub label: String,
    pub cadence: String,
    pub cwd: Option<String>,
    pub runs: u64,
    pub failures: u64,
    pub last_run: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub output_tokens: u64,
    pub energy_wh: f64,
    pub observations: Vec<String>,
}

pub fn id_for(source: &Source) -> String {
    format!("{:06x}", fnv1a(source.key().as_bytes()) & 0xff_ffff)
}

/// Everything discovered on this machine, with sessions attached to the
/// launcher that produced them where that can be told.
pub fn discover(sessions: &[AiSession], home: &Path) -> Vec<Entry> {
    let crontab = read_crontab_text();
    let agents = read_launch_agents(home);
    let jobs = read_daemon_jobs(home);
    discover_with(sessions, &crontab, &agents, &jobs)
}

/// The discovery logic over explicit inputs, so tests never read the real
/// crontab, LaunchAgents, or daemon jobs.
pub fn discover_with(
    sessions: &[AiSession],
    crontab_text: &str,
    launch_agents: &[(String, String)],
    daemon_jobs: &[Source],
) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();

    // Automated sessions grouped by (cwd, first prompt): a job signature.
    let mut by_signature: BTreeMap<(String, String), Vec<&AiSession>> = BTreeMap::new();
    for s in sessions.iter().filter(|s| s.launch.is_automated()) {
        let cwd = s.project_path.clone().unwrap_or_default();
        let prompt = s.launch.first_prompt.clone().unwrap_or_default();
        by_signature.entry((cwd, prompt)).or_default().push(s);
    }

    // Cron entries that invoke claude, directly or through a script.
    let crons: Vec<(String, String, String, Option<String>)> = parse_crontab(crontab_text)
        .into_iter()
        .map(|(line, schedule, command)| {
            let body = script_body(&command);
            (line, schedule, command, body)
        })
        .filter(|(_, _, command, body)| {
            command.contains("claude") || body.as_deref().is_some_and(|t| t.contains("claude"))
        })
        .collect();

    // Attach signatures to cron entries only when the prompt appears in
    // the cron command or the script it runs. Matching on the working
    // directory would fold two scripts in one project into each other, and
    // would re-attach orphaned runs to whichever entry happens to remain.
    let mut assigned: Vec<Vec<(String, String)>> = vec![Vec::new(); crons.len()];
    let mut claimed: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for (key, _) in &by_signature {
        let (_, prompt) = key;
        if prompt.is_empty() {
            continue;
        }
        let head: String = prompt.chars().take(40).collect();
        let target = crons.iter().position(|(_, _, command, body)| {
            command.contains(prompt.as_str())
                || body
                    .as_deref()
                    .is_some_and(|t| t.contains(prompt.as_str()) || t.contains(head.as_str()))
        });
        if let Some(i) = target {
            assigned[i].push(key.clone());
            claimed.insert(key.clone());
        }
    }

    for (i, (line, schedule, command, _)) in crons.iter().enumerate() {
        let source = Source::Cron {
            line: line.clone(),
            schedule: schedule.clone(),
            command: command.clone(),
        };
        let matched: Vec<&AiSession> = assigned[i]
            .iter()
            .flat_map(|k| by_signature.get(k).into_iter().flatten().copied())
            .collect();
        let label = matched
            .first()
            .and_then(|s| s.launch.first_prompt.clone())
            .unwrap_or_else(|| command.clone());
        let mut e = entry_from_runs(source, label, &matched);
        e.cadence = describe_cron(schedule);
        e.cwd = matched.first().and_then(|s| s.project_path.clone());
        entries.push(e);
    }
    by_signature.retain(|k, _| !claimed.contains(k));

    // launchd agents mentioning claude.
    for (path, label) in launch_agents.iter().cloned() {
        let source = Source::Launchd {
            path: path.clone(),
            label: label.clone(),
        };
        let mut e = entry_from_runs(source, label, &[]);
        e.cadence = "launchd agent".into();
        entries.push(e);
    }

    // Claude Code daemon jobs.
    for job in daemon_jobs.iter().cloned() {
        let name = match &job {
            Source::DaemonJob { name, .. } => name.clone(),
            _ => unreachable!(),
        };
        let mut e = entry_from_runs(job.clone(), name, &[]);
        e.cadence = "background job".into();
        if let Source::DaemonJob {
            state,
            needs,
            created_at,
            ..
        } = &job
        {
            e.cwd = None;
            let mut obs = format!("state {}", state);
            if let Some(n) = needs {
                obs.push_str(&format!(", needs: {}", n));
            }
            if let Some(c) = created_at {
                let days = (Utc::now() - *c).num_days();
                obs.push_str(&format!(", for {} days", days));
            }
            e.observations.push(obs);
        }
        entries.push(e);
    }

    // Whatever automated signatures no launcher claimed.
    for ((cwd, prompt), runs) in by_signature {
        let source = Source::SdkSignature {
            cwd: cwd.clone(),
            prompt: prompt.clone(),
        };
        let label = if prompt.is_empty() {
            "(no prompt recorded)".to_string()
        } else {
            prompt.clone()
        };
        let mut e = entry_from_runs(source, label, &runs);
        e.cadence = infer_cadence(&runs);
        e.cwd = Some(cwd);
        e.observations
            .push("launched by a program; no cron or launchd entry found for it".into());
        entries.push(e);
    }

    entries.sort_by(|a, b| b.runs.cmp(&a.runs).then(a.label.cmp(&b.label)));
    entries
}

fn is_failure(s: &AiSession) -> bool {
    s.launch.api_error.is_some() || (s.model.is_none() && s.tokens.total() == 0)
}

fn entry_from_runs(source: Source, label: String, runs: &[&AiSession]) -> Entry {
    let failures: Vec<&&AiSession> = runs.iter().filter(|s| is_failure(s)).collect();
    let last_run = runs.iter().filter_map(|s| s.started_at).max();
    let last_success = runs
        .iter()
        .filter(|s| !is_failure(s))
        .filter_map(|s| s.started_at)
        .max();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for s in &failures {
        let r = s
            .launch
            .api_error
            .clone()
            .unwrap_or_else(|| "no model output".into());
        *reasons.entry(r).or_default() += 1;
    }
    let failure_reason = reasons
        .iter()
        .max_by_key(|(_, n)| **n)
        .map(|(r, _)| r.clone());

    let owned: Vec<AiSession> = runs.iter().map(|s| (*s).clone()).collect();
    let est = energy::estimate_total_energy(&owned, &EnergyConfig::default());

    let mut observations = Vec::new();
    if !runs.is_empty() && failures.len() == runs.len() {
        observations.push(format!(
            "has never succeeded ({} run{})",
            runs.len(),
            if runs.len() == 1 { "" } else { "s" }
        ));
    } else if !failures.is_empty() {
        observations.push(format!("{} of {} runs failed", failures.len(), runs.len()));
    }
    if let (Some(reason), true) = (&failure_reason, !failures.is_empty()) {
        observations.push(format!("failure reason: {}", reason));
    }
    if let Some(last) = last_run {
        let days = (Utc::now() - last).num_days();
        if days > 30 {
            observations.push(format!("no runs in the last {} days", days));
        }
    }

    Entry {
        id: id_for(&source),
        source,
        label,
        cadence: String::new(),
        cwd: None,
        runs: runs.len() as u64,
        failures: failures.len() as u64,
        last_run,
        last_success,
        failure_reason,
        output_tokens: runs.iter().map(|s| s.tokens.output).sum(),
        energy_wh: est.total_wh,
        observations,
    }
}

/// The user's crontab, or empty when there is none or `crontab` is missing.
pub fn read_crontab_text() -> String {
    match std::process::Command::new("crontab").arg("-l").output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    }
}

/// Pure crontab parsing, for tests.
pub fn parse_crontab(text: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.contains('=')
                && !line.starts_with(|c: char| c.is_ascii_digit() || c == '*' || c == '@')
        {
            continue;
        }
        let (schedule, command) = if let Some(rest) = line.strip_prefix('@') {
            match rest.split_once(char::is_whitespace) {
                Some((s, c)) => (format!("@{}", s), c.trim().to_string()),
                None => continue,
            }
        } else {
            let parts: Vec<&str> = line.splitn(6, char::is_whitespace).collect();
            if parts.len() < 6 {
                continue;
            }
            (parts[..5].join(" "), parts[5].trim().to_string())
        };
        out.push((raw.to_string(), schedule, command));
    }
    out
}

/// Human wording for the common cron shapes; the raw expression otherwise.
pub fn describe_cron(schedule: &str) -> String {
    let f: Vec<&str> = schedule.split_whitespace().collect();
    if f.len() != 5 {
        return schedule.to_string();
    }
    let (m, h, dom, mon, dow) = (f[0], f[1], f[2], f[3], f[4]);
    let time = match (m.parse::<u32>(), h.parse::<u32>()) {
        (Ok(m), Ok(h)) => format!("{:02}:{:02}", h, m),
        _ => return schedule.to_string(),
    };
    match (dom, mon, dow) {
        ("*", "*", "*") => format!("daily at {}", time),
        ("*", "*", "1-5") => format!("weekdays at {}", time),
        ("*", "*", "0,6") | ("*", "*", "6,0") => format!("weekends at {}", time),
        ("*", "*", d) => format!("at {} on days {}", time, d),
        _ => schedule.to_string(),
    }
}

/// The body of the script a cron command runs, when the first token is a
/// readable file. Lets a `run-digest.sh` be matched to its sessions.
fn script_body(command: &str) -> Option<String> {
    let first = command.split_whitespace().next()?;
    let p = Path::new(first);
    if p.is_file() {
        std::fs::read_to_string(p).ok()
    } else {
        None
    }
}

fn read_launch_agents(home: &Path) -> Vec<(String, String)> {
    let dir = home.join("Library").join("LaunchAgents");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "plist") {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    if text.contains("claude") {
                        let label = p
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        out.push((p.to_string_lossy().to_string(), label));
                    }
                }
            }
        }
    }
    out
}

fn read_daemon_jobs(home: &Path) -> Vec<Source> {
    let dir = home.join(".claude").join("jobs");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path().join("state.json");
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            let get = |k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
            out.push(Source::DaemonJob {
                dir: e.path().to_string_lossy().to_string(),
                name: get("name").unwrap_or_else(|| "(unnamed)".into()),
                state: get("state").unwrap_or_else(|| "unknown".into()),
                needs: get("needs"),
                created_at: get("createdAt").and_then(|s| s.parse().ok()),
            });
        }
    }
    out
}

/// From run timestamps: "every ~24h", "every ~7d", or "irregular".
pub fn infer_cadence(runs: &[&AiSession]) -> String {
    let mut starts: Vec<DateTime<Utc>> = runs.iter().filter_map(|s| s.started_at).collect();
    if starts.len() < 2 {
        return "one run".into();
    }
    starts.sort();
    let mut gaps: Vec<i64> = starts
        .windows(2)
        .map(|w| (w[1] - w[0]).num_minutes())
        .collect();
    gaps.sort();
    let median = gaps[gaps.len() / 2];
    let hours = median as f64 / 60.0;
    if hours < 1.0 {
        format!("every ~{} min", median)
    } else if hours < 36.0 {
        format!("every ~{:.0}h", hours)
    } else {
        format!("every ~{:.0}d", hours / 24.0)
    }
}

/// What `prune` would do for an entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PruneAction {
    RemoveCronLine { line: String },
    UnloadLaunchd { path: String, moved_to: String },
    ArchiveDaemonJob { dir: String, moved_to: String },
    NothingToRemove { reason: String },
}

pub fn plan(entry: &Entry, home: &Path) -> PruneAction {
    let pruned = home.join(".claude").join("pruned");
    match &entry.source {
        Source::Cron { line, .. } => PruneAction::RemoveCronLine { line: line.clone() },
        Source::Launchd { path, label } => PruneAction::UnloadLaunchd {
            path: path.clone(),
            moved_to: pruned
                .join(format!("{}.plist", label))
                .to_string_lossy()
                .to_string(),
        },
        Source::DaemonJob { dir, .. } => PruneAction::ArchiveDaemonJob {
            dir: dir.clone(),
            moved_to: pruned
                .join(Path::new(dir).file_name().unwrap_or_default())
                .to_string_lossy()
                .to_string(),
        },
        Source::SdkSignature { cwd, .. } => PruneAction::NothingToRemove {
            reason: format!(
                "these runs were started by a program, but no cron or launchd entry was found; look for what runs `claude -p` in {}",
                cwd
            ),
        },
    }
}

/// Crontab text without the given line. Matches the exact line so a
/// renumbered crontab cannot remove the wrong entry. Returns whether
/// anything was removed.
pub fn remove_cron_line(text: &str, line: &str) -> (String, bool) {
    let mut removed = false;
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| {
            if !removed && l.trim() == line.trim() {
                removed = true;
                false
            } else {
                true
            }
        })
        .collect();
    let mut out = kept.join("\n");
    if text.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    (out, removed)
}

/// Carry out a plan. Session logs are never touched.
pub fn apply(action: &PruneAction) -> Result<String> {
    match action {
        PruneAction::RemoveCronLine { line } => {
            let current = std::process::Command::new("crontab")
                .arg("-l")
                .output()
                .map_err(|e| {
                    ConscienceError::Other(anyhow::anyhow!("cannot read crontab: {}", e))
                })?;
            if !current.status.success() {
                return Err(ConscienceError::Other(anyhow::anyhow!("crontab -l failed")));
            }
            let text = String::from_utf8_lossy(&current.stdout).to_string();
            let (new_text, removed) = remove_cron_line(&text, line);
            if !removed {
                return Err(ConscienceError::Other(anyhow::anyhow!(
                    "that line is no longer in the crontab; nothing changed"
                )));
            }
            // Keep a copy of what was there before writing.
            let backup = backup_path("crontab");
            std::fs::create_dir_all(backup.parent().unwrap()).ok();
            std::fs::write(&backup, &text).ok();
            use std::io::Write as _;
            let mut child = std::process::Command::new("crontab")
                .arg("-")
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| {
                    ConscienceError::Other(anyhow::anyhow!("cannot write crontab: {}", e))
                })?;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(new_text.as_bytes())
                .map_err(|e| {
                    ConscienceError::Other(anyhow::anyhow!("cannot write crontab: {}", e))
                })?;
            let status = child
                .wait()
                .map_err(|e| ConscienceError::Other(anyhow::anyhow!("{}", e)))?;
            if !status.success() {
                return Err(ConscienceError::Other(anyhow::anyhow!("crontab - failed")));
            }
            Ok(format!(
                "removed the cron line; previous crontab saved to {}",
                backup.display()
            ))
        }
        PruneAction::UnloadLaunchd { path, moved_to } => {
            let _ = std::process::Command::new("launchctl")
                .args(["unload", path])
                .status();
            std::fs::create_dir_all(Path::new(moved_to).parent().unwrap()).ok();
            std::fs::rename(path, moved_to).map_err(|e| {
                ConscienceError::Other(anyhow::anyhow!("cannot move {}: {}", path, e))
            })?;
            Ok(format!("unloaded and moved the agent to {}", moved_to))
        }
        PruneAction::ArchiveDaemonJob { dir, moved_to } => {
            std::fs::create_dir_all(Path::new(moved_to).parent().unwrap()).ok();
            std::fs::rename(dir, moved_to).map_err(|e| {
                ConscienceError::Other(anyhow::anyhow!("cannot move {}: {}", dir, e))
            })?;
            Ok(format!("moved the job to {}", moved_to))
        }
        PruneAction::NothingToRemove { reason } => {
            Err(ConscienceError::Other(anyhow::anyhow!("{}", reason)))
        }
    }
}

fn backup_path(what: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("pruned")
        .join(format!(
            "{}-{}.bak",
            what,
            Utc::now().format("%Y%m%dT%H%M%SZ")
        ))
}

pub fn describe_action(action: &PruneAction) -> String {
    match action {
        PruneAction::RemoveCronLine { line } => {
            format!("remove this line from your crontab:\n    {}", line)
        }
        PruneAction::UnloadLaunchd { path, moved_to } => {
            format!("unload {} and move it to {}", path, moved_to)
        }
        PruneAction::ArchiveDaemonJob { dir, moved_to } => format!("move {} to {}", dir, moved_to),
        PruneAction::NothingToRemove { reason } => format!("nothing to remove: {}", reason),
    }
}

/// Text rendering for the terminal.
pub fn render(entries: &[Entry]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  Conscience \u{2014} Automation");
    let _ = writeln!(out);
    if entries.is_empty() {
        let _ = writeln!(
            out,
            "  Nothing found that runs Claude without a person present."
        );
        let _ = writeln!(out);
        return out;
    }
    let _ = writeln!(
        out,
        "  {} thing(s) run without a person present. Observations describe the data; the decision is yours.",
        entries.len()
    );
    let _ = writeln!(out);
    for e in entries {
        let _ = writeln!(out, "  {}  [{}]  {}", e.id, e.source.kind(), e.label);
        let _ = writeln!(out, "         cadence:  {}", e.cadence);
        if let Some(c) = &e.cwd {
            let _ = writeln!(out, "         in:       {}", c);
        }
        if e.runs > 0 {
            let _ = writeln!(
                out,
                "         runs:     {} ({} failed){}{}",
                e.runs,
                e.failures,
                e.last_run
                    .map(|t| format!(", last {}", t.format("%Y-%m-%d %H:%M")))
                    .unwrap_or_default(),
                match e.last_success {
                    Some(t) => format!(", last success {}", t.format("%Y-%m-%d")),
                    None => ", last success never".into(),
                }
            );
            let _ = writeln!(
                out,
                "         cost:     {} output tokens, ~{:.0} Wh",
                e.output_tokens, e.energy_wh
            );
        }
        for o in &e.observations {
            let _ = writeln!(out, "         \u{2022} {}", o);
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "  To remove a launcher: conscience prune <id>   (shows the plan and asks first; --dry-run to only show it)"
    );
    let _ = writeln!(out);
    out
}
