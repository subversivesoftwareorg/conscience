use crate::ethics::models::{Principle, ReflectionQuestion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{BufRead, Write};

/// A saved reflection session with metadata for longitudinal tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionSession {
    pub timestamp: String,
    pub contributor: Option<String>,
    pub project: Option<String>,
    pub responses: Vec<ReflectionResponse>,
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
