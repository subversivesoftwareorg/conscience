use crate::ethics::models::{Principle, ReflectionQuestion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{BufRead, Write};

/// A saved reflection session with metadata for longitudinal tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionSession {
    /// Unique per session, e.g. `20260919T093012Z-a1b2c3`. Also the default
    /// filename stem. `None` for files saved before IDs existed, which were
    /// named by date alone and could overwrite each other.
    #[serde(default)]
    pub session_id: Option<String>,
    pub timestamp: String,
    pub contributor: Option<String>,
    pub project: Option<String>,
    pub responses: Vec<ReflectionResponse>,
}

impl ReflectionSession {
    /// Start a new session stamped with the current time and a fresh ID.
    pub fn new(
        contributor: Option<String>,
        project: Option<String>,
        responses: Vec<ReflectionResponse>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            session_id: Some(new_session_id(now)),
            timestamp: now.to_rfc3339(),
            contributor,
            project,
            responses,
        }
    }

    /// Default filename for this session inside a reflections directory.
    /// Falls back to the date for legacy records with no ID.
    pub fn default_filename(&self) -> String {
        match &self.session_id {
            Some(id) => format!("{}.json", id),
            None => format!("{}.json", &self.timestamp[..10.min(self.timestamp.len())]),
        }
    }
}

/// Timestamp to the second plus six hex characters of entropy drawn from
/// the nanosecond clock and the process ID. Two sessions saved in the same
/// second by the same person still get distinct names, without pulling in
/// a UUID dependency for one identifier.
fn new_session_id(now: chrono::DateTime<chrono::Utc>) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    now.timestamp_nanos_opt().unwrap_or_default().hash(&mut h);
    std::process::id().hash(&mut h);
    // Distinguish rapid successive calls within one process as well.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    COUNTER
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .hash(&mut h);
    format!(
        "{}-{:06x}",
        now.format("%Y%m%dT%H%M%SZ"),
        h.finish() & 0xff_ffff
    )
}

/// One answered (or skipped) reflection question from an interactive session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResponse {
    pub principle: Principle,
    pub question: String,
    pub answer: Option<String>,
}

/// Run an interactive reflection session: present each question, collect a
/// multi-line answer terminated by a blank line. An empty first line skips
/// the question; EOF skips everything remaining.
pub fn run_session(
    questions: &[ReflectionQuestion],
    mut input: impl BufRead,
    mut out: impl Write,
) -> Vec<ReflectionResponse> {
    let mut responses = Vec::with_capacity(questions.len());
    let mut eof = false;

    for (i, q) in questions.iter().enumerate() {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  {} of {}. {}",
            i + 1,
            questions.len(),
            q.principle.name()
        );
        let _ = writeln!(out, "     Source: {}", q.principle.source());
        let _ = writeln!(out, "     Data: {}", q.data_context);
        let _ = writeln!(out, "     Q: {}", q.question);

        let answer = if eof {
            None
        } else {
            let _ = writeln!(
                out,
                "     Your answer (blank line to finish, Enter to skip):"
            );
            let _ = out.flush();
            read_answer(&mut input, &mut eof)
        };

        responses.push(ReflectionResponse {
            principle: q.principle,
            question: q.question.clone(),
            answer,
        });
    }

    responses
}

/// Read lines until a blank line or EOF. Returns None if no content was
/// entered (skip); sets `eof` when input is exhausted.
fn read_answer(input: &mut impl BufRead, eof: &mut bool) -> Option<String> {
    let mut lines = Vec::new();

    loop {
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => {
                *eof = true;
                break;
            }
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\n', '\r']);
                if trimmed.is_empty() {
                    break;
                }
                lines.push(trimmed.to_string());
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Render the end-of-session summary of answers and skips.
pub fn render_summary(responses: &[ReflectionResponse]) -> String {
    let answered = responses.iter().filter(|r| r.answer.is_some()).count();

    let mut out = String::new();
    let _ = writeln!(out, "  Session Summary");
    let _ = writeln!(out, "  {} of {} answered", answered, responses.len());
    let _ = writeln!(out);

    for r in responses {
        let _ = writeln!(out, "  {}", r.principle.name());
        let _ = writeln!(out, "     Q: {}", r.question);
        match &r.answer {
            Some(a) => {
                for line in a.lines() {
                    let _ = writeln!(out, "     A: {}", line);
                }
            }
            None => {
                let _ = writeln!(out, "     A: (skipped)");
            }
        }
        let _ = writeln!(out);
    }

    out
}

/// One contributor's answer to a question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorAnswer {
    pub contributor: Option<String>,
    pub timestamp: String,
    pub answer: String,
}

/// Aggregated view of a single principle across all sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleAggregate {
    pub principle: Principle,
    pub question: String,
    pub answers: Vec<ContributorAnswer>,
    pub skipped: u64,
}

/// Aggregated view across multiple reflection sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetroAggregate {
    pub session_count: u64,
    pub contributors: Vec<String>,
    pub by_principle: Vec<PrincipleAggregate>,
}

pub fn aggregate_sessions(sessions: &[ReflectionSession]) -> RetroAggregate {
    let mut contributors: Vec<String> = Vec::new();
    let mut by_key: BTreeMap<String, PrincipleAggregate> = BTreeMap::new();

    for session in sessions {
        if let Some(c) = &session.contributor {
            if !contributors.contains(c) {
                contributors.push(c.clone());
            }
        }

        for r in &session.responses {
            let key = format!("{:?}:{}", r.principle, r.question);
            let entry = by_key.entry(key).or_insert_with(|| PrincipleAggregate {
                principle: r.principle,
                question: r.question.clone(),
                answers: Vec::new(),
                skipped: 0,
            });

            match &r.answer {
                Some(text) => {
                    entry.answers.push(ContributorAnswer {
                        contributor: session.contributor.clone(),
                        timestamp: session.timestamp.clone(),
                        answer: text.clone(),
                    });
                }
                None => {
                    entry.skipped += 1;
                }
            }
        }
    }

    RetroAggregate {
        session_count: sessions.len() as u64,
        contributors,
        by_principle: by_key.into_values().collect(),
    }
}
