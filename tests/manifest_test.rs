use conscience::ethics::manifest::*;

#[test]
fn test_minimal_manifest_parses() {
    let yaml = r#"
project:
  name: "Test Project"
"#;
    let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.project.name, "Test Project");
    assert!(manifest.project.beneficiaries.is_empty());
    assert!(manifest.thresholds.solo_project == false);
}

#[test]
fn test_full_manifest_parses() {
    let yaml = r#"
project:
  name: "Full Project"
  mission: "Test the parser"
  beneficiaries:
    - name: "Users"
      description: "End users"
  cost_bearers:
    - name: "Team"
      description: "Engineering team"

github:
  repo: "org/repo"

value_categories:
  - name: "revenue"
    value_type: revenue
    description: "Money stuff"

team:
  size: 5
  roles:
    senior: 2
    mid: 2
    junior: 1
  learning_goals:
    - "Learn Rust"

monthly_review:
  last_updated: "2026-06-01"
  value_delivered: "Shipped v1"
  revenue_impact: "$100K"
  growth_observations: "Team is learning"
  ethical_notes: "All good"
  ai_sentiment: "mostly_positive"

thresholds:
  contribution_concentration_warn: 0.90
  solo_project: true
  tokens_per_file_warn: 200000
  max_session_hours: 24.0
"#;
    let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.project.name, "Full Project");
    assert_eq!(manifest.github.repo, Some("org/repo".to_string()));
    assert_eq!(manifest.team.size, 5);
    assert_eq!(manifest.team.roles.junior, 1);
    assert_eq!(manifest.team.learning_goals.len(), 1);
    assert_eq!(manifest.monthly_review.ai_sentiment, "mostly_positive");
    assert!(manifest.thresholds.solo_project);
    assert_eq!(manifest.thresholds.contribution_concentration_warn, 0.90);
    assert_eq!(manifest.thresholds.tokens_per_file_warn, 200_000);
    assert_eq!(manifest.thresholds.max_session_hours, 24.0);
    assert_eq!(manifest.value_categories[0].value_type, ValueType::Revenue);
}

#[test]
fn test_defaults_applied() {
    let yaml = "{}";
    let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(manifest.thresholds.contribution_concentration_warn, 0.80);
    assert_eq!(manifest.thresholds.ai_dependency_concern, 12.0);
    assert_eq!(manifest.thresholds.tokens_per_file_warn, 100_000);
    assert_eq!(manifest.thresholds.max_session_hours, 12.0);
    assert!(!manifest.thresholds.solo_project);
}

#[test]
fn test_monthly_review_staleness() {
    let review = MonthlyReview {
        last_updated: "2026-05-01".to_string(),
        ..Default::default()
    };
    assert!(review.is_stale("2026-06-04"));
    assert!(!review.is_stale("2026-05-15"));
}

#[test]
fn test_monthly_review_never_updated_is_stale() {
    let review = MonthlyReview::default();
    assert!(review.is_stale("2026-06-04"));
}

#[test]
fn test_monthly_review_populated() {
    let empty = MonthlyReview::default();
    assert!(!empty.is_populated());

    let populated = MonthlyReview {
        value_delivered: "Something".to_string(),
        ..Default::default()
    };
    assert!(populated.is_populated());
}

#[test]
fn attention_thresholds_default_and_parse() {
    let manifest: Manifest = serde_yaml::from_str("{}").unwrap();
    let a = &manifest.thresholds.attention;
    assert_eq!(a.idle_minutes, 15.0);
    assert_eq!(a.engagement_floor_minutes, 2.0);
    assert_eq!(a.flow_gap_minutes, 10.0);
    assert_eq!(a.flow_min_minutes, 20.0);
    assert!(a.project_aliases.is_empty());

    let yaml = r#"
thresholds:
  attention:
    idle_minutes: 20
    project_aliases:
      "/tmp/worktrees/*": conscience
"#;
    let m: Manifest = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(m.thresholds.attention.idle_minutes, 20.0);
    assert_eq!(m.thresholds.attention.flow_gap_minutes, 10.0, "unset keeps default");
    assert_eq!(
        m.thresholds.attention.project_aliases.get("/tmp/worktrees/*").map(String::as_str),
        Some("conscience")
    );
}

#[test]
fn ai_dependency_defaults_recalibrated_for_corrected_counting() {
    let manifest: Manifest = serde_yaml::from_str("{}").unwrap();
    assert_eq!(manifest.thresholds.ai_dependency_info, 6.0);
    assert_eq!(manifest.thresholds.ai_dependency_concern, 12.0);
}
