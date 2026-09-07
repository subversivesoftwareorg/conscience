use conscience::ethics::models::*;
use conscience::ethics::reflection;
use conscience::ethics::report;

#[test]
fn generate_reflections_with_no_data_returns_base_questions() {
    let questions = reflection::generate_reflections(None, None, None);

    // The six base questions are always present; agent-autonomy and
    // revenue questions require data, so they must not appear.
    assert_eq!(questions.len(), 6);
    for q in &questions {
        assert!(
            q.data_context.contains("No data")
                || q.data_context.contains("No AI usage data")
                || q.data_context.contains("No GitHub data"),
            "expected a no-data fallback context, got: {}",
            q.data_context
        );
        assert!(!q.question.is_empty());
    }
}

#[test]
fn who_benefits_context_has_no_double_period_when_manifest_present() {
    let yaml = r#"
project:
  name: "Test"
  beneficiaries:
    - name: "Users"
      description: "End users"
"#;
    let manifest: conscience::ethics::manifest::Manifest = serde_yaml::from_str(yaml).unwrap();

    let questions = reflection::generate_reflections(None, None, Some(&manifest));

    let who_benefits = questions
        .iter()
        .find(|q| q.data_context.contains("Stated beneficiaries"))
        .expect("beneficiaries should appear in a data context");
    assert!(
        !who_benefits.data_context.contains(".."),
        "double period in context: {}",
        who_benefits.data_context
    );
}

#[test]
fn render_reflection_session_includes_question_details() {
    let questions = vec![ReflectionQuestion {
        principle: Principle::DeveloperGrowth,
        data_context: "3 contributors made 42 commits.".to_string(),
        question: "Are team members learning new skills?".to_string(),
    }];

    let output = report::render_reflection_session(&questions);

    assert!(output.contains("1."), "questions should be numbered");
    assert!(output.contains("Developer Growth"), "principle name shown");
    assert!(
        output.contains("MH 52"),
        "source citation shown, got:\n{}",
        output
    );
    assert!(output.contains("3 contributors made 42 commits."));
    assert!(output.contains("Are team members learning new skills?"));
}

#[test]
fn render_reflection_session_frames_for_team_discussion() {
    let output = report::render_reflection_session(&[]);

    assert!(
        output.contains("not automated judgment"),
        "header should frame questions as discussion prompts, got:\n{}",
        output
    );
}
