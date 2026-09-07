use conscience::ethics::models::*;
use conscience::ethics::session::{ReflectionResponse, render_summary, run_session};

fn make_questions(n: usize) -> Vec<ReflectionQuestion> {
    let principles = [
        Principle::DeveloperGrowth,
        Principle::Transparency,
        Principle::Security,
    ];
    (0..n)
        .map(|i| ReflectionQuestion {
            principle: principles[i % principles.len()],
            data_context: format!("Context {}.", i + 1),
            question: format!("Question number {}?", i + 1),
        })
        .collect()
}

#[test]
fn multi_line_answer_ends_at_blank_line() {
    let questions = make_questions(1);
    let input = b"first line\nsecond line\n\n" as &[u8];
    let mut out = Vec::new();

    let responses = run_session(&questions, input, &mut out);

    assert_eq!(responses.len(), 1);
    assert_eq!(
        responses[0].answer.as_deref(),
        Some("first line\nsecond line")
    );
}

#[test]
fn empty_first_line_skips_question() {
    let questions = make_questions(2);
    let input = b"\nan answer\n\n" as &[u8];
    let mut out = Vec::new();

    let responses = run_session(&questions, input, &mut out);

    assert_eq!(responses.len(), 2);
    assert!(responses[0].answer.is_none(), "first should be skipped");
    assert_eq!(responses[1].answer.as_deref(), Some("an answer"));
}

#[test]
fn eof_marks_remaining_questions_skipped() {
    let questions = make_questions(3);
    let input = b"only answer\n\n" as &[u8];
    let mut out = Vec::new();

    let responses = run_session(&questions, input, &mut out);

    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0].answer.as_deref(), Some("only answer"));
    assert!(responses[1].answer.is_none());
    assert!(responses[2].answer.is_none());
}

#[test]
fn session_prompts_show_question_and_source() {
    let questions = make_questions(1);
    let input = b"\n" as &[u8];
    let mut out = Vec::new();

    run_session(&questions, input, &mut out);

    let printed = String::from_utf8(out).unwrap();
    assert!(printed.contains("Question number 1?"));
    assert!(printed.contains("Developer Growth"));
    assert!(printed.contains("MH 52"), "source citation shown");
    assert!(printed.contains("Context 1."));
}

#[test]
fn summary_shows_answers_skips_and_count() {
    let responses = vec![
        ReflectionResponse {
            principle: Principle::DeveloperGrowth,
            question: "Q1?".to_string(),
            answer: Some("We paired on the parser.".to_string()),
        },
        ReflectionResponse {
            principle: Principle::Transparency,
            question: "Q2?".to_string(),
            answer: None,
        },
    ];

    let summary = render_summary(&responses);

    assert!(summary.contains("We paired on the parser."));
    assert!(summary.contains("(skipped)"));
    assert!(summary.contains("1 of 2 answered"), "got:\n{}", summary);
}

#[test]
fn responses_serialize_to_json() {
    let responses = vec![ReflectionResponse {
        principle: Principle::Security,
        question: "Q?".to_string(),
        answer: Some("Yes.".to_string()),
    }];

    let json = serde_json::to_string(&responses).unwrap();

    assert!(json.contains("\"security\""));
    assert!(json.contains("\"Yes.\""));
}
