use conscience::ethics::models::Principle;
use conscience::ethics::session::{ReflectionResponse, ReflectionSession};

#[test]
fn session_serializes_with_metadata() {
    let session = ReflectionSession {
        session_id: None,
        timestamp: "2026-09-14T12:00:00Z".to_string(),
        contributor: Some("alice".to_string()),
        project: Some("conscience".to_string()),
        responses: vec![
            ReflectionResponse {
                principle: Principle::DeveloperGrowth,
                question: "Are team members learning?".to_string(),
                answer: Some("Yes, we paired on the parser.".to_string()),
            },
            ReflectionResponse {
                principle: Principle::Transparency,
                question: "Is AI involvement disclosed?".to_string(),
                answer: None,
            },
        ],
    };

    let json = serde_json::to_string_pretty(&session).unwrap();
    assert!(json.contains("\"contributor\": \"alice\""));
    assert!(json.contains("\"developer_growth\""));
    assert!(json.contains("\"answer\": null"));

    let parsed: ReflectionSession = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.contributor.as_deref(), Some("alice"));
    assert_eq!(parsed.responses.len(), 2);
    assert_eq!(parsed.responses[0].answer.as_deref(), Some("Yes, we paired on the parser."));
}

#[test]
fn aggregate_groups_by_principle() {
    use conscience::ethics::session::aggregate_sessions;

    let s1 = ReflectionSession {
        session_id: None,
        timestamp: "2026-09-14T12:00:00Z".to_string(),
        contributor: Some("alice".to_string()),
        project: Some("conscience".to_string()),
        responses: vec![ReflectionResponse {
            principle: Principle::DeveloperGrowth,
            question: "Are team members learning?".to_string(),
            answer: Some("Yes.".to_string()),
        }],
    };
    let s2 = ReflectionSession {
        session_id: None,
        timestamp: "2026-09-14T13:00:00Z".to_string(),
        contributor: Some("bob".to_string()),
        project: Some("conscience".to_string()),
        responses: vec![ReflectionResponse {
            principle: Principle::DeveloperGrowth,
            question: "Are team members learning?".to_string(),
            answer: Some("Not sure.".to_string()),
        }],
    };

    let agg = aggregate_sessions(&[s1, s2]);
    assert_eq!(agg.session_count, 2);
    assert_eq!(agg.contributors.len(), 2);

    let growth = agg.by_principle.iter().find(|p| p.principle == Principle::DeveloperGrowth).unwrap();
    assert_eq!(growth.answers.len(), 2);
    assert_eq!(growth.answers[0].contributor.as_deref(), Some("alice"));
    assert_eq!(growth.answers[1].contributor.as_deref(), Some("bob"));
    assert_eq!(growth.skipped, 0);
}

#[test]
fn aggregate_counts_skips() {
    use conscience::ethics::session::aggregate_sessions;

    let s1 = ReflectionSession {
        session_id: None,
        timestamp: "2026-09-14T12:00:00Z".to_string(),
        contributor: Some("alice".to_string()),
        project: None,
        responses: vec![ReflectionResponse {
            principle: Principle::Transparency,
            question: "Is AI involvement disclosed?".to_string(),
            answer: None,
        }],
    };

    let agg = aggregate_sessions(&[s1]);
    let transparency = agg.by_principle.iter().find(|p| p.principle == Principle::Transparency).unwrap();
    assert_eq!(transparency.answers.len(), 0);
    assert_eq!(transparency.skipped, 1);
}

#[test]
fn new_sessions_get_unique_ids_and_filenames() {
    let a = ReflectionSession::new(Some("alice".into()), None, vec![]);
    let b = ReflectionSession::new(Some("alice".into()), None, vec![]);

    let id_a = a.session_id.clone().expect("new sessions carry an id");
    let id_b = b.session_id.clone().expect("new sessions carry an id");
    assert_ne!(id_a, id_b, "two sessions in the same second must not collide");

    // Filename is the id, so two saves in one day land in two files.
    assert_eq!(a.default_filename(), format!("{}.json", id_a));
    assert_ne!(a.default_filename(), b.default_filename());

    // Shape: YYYYMMDDTHHMMSSZ-xxxxxx, sortable by time.
    assert_eq!(id_a.len(), 16 + 1 + 6, "{}", id_a);
    assert_eq!(&id_a[8..9], "T");
    assert_eq!(&id_a[15..17], "Z-");
}

#[test]
fn legacy_records_without_id_still_load_and_name_by_date() {
    let json = r#"{"timestamp":"2026-09-14T12:00:00Z","contributor":"alice","project":null,"responses":[]}"#;
    let s: ReflectionSession = serde_json::from_str(json).unwrap();
    assert_eq!(s.session_id, None);
    assert_eq!(s.default_filename(), "2026-09-14.json");
}
