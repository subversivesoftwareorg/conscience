use crate::ai_tools::models::AiUsageSummary;
use crate::ethics::manifest::Manifest;
use crate::ethics::models::*;
use crate::github::models::RepoSummary;
use std::collections::HashMap;

/// Generate reflection questions populated with real data.
/// These are for team retrospectives — not automated judgments.
pub fn generate_reflections(
    github: Option<&RepoSummary>,
    ai: Option<&AiUsageSummary>,
    manifest: Option<&Manifest>,
) -> Vec<ReflectionQuestion> {
    let mut questions = vec![
        build_human_growth_reflection(github, ai, manifest),
        build_who_benefits_reflection(github, manifest),
        build_transparency_reflection(ai),
        build_proportionality_reflection(github, ai, manifest),
        build_babel_or_jerusalem_reflection(github, ai),
        build_security_reflection(ai),
    ];

    if let Some(ai_data) = ai {
        if ai_data.agent_actions_summary.total_actions > 0 {
            questions.push(build_agent_autonomy_reflection(ai_data));
        }
    }

    if let Some(m) = manifest {
        if !m.monthly_review.revenue_impact.is_empty() {
            questions.push(build_revenue_reflection(m));
        }
    }

    questions
}

fn build_human_growth_reflection(
    github: Option<&RepoSummary>,
    ai: Option<&AiUsageSummary>,
    manifest: Option<&Manifest>,
) -> ReflectionQuestion {
    let mut context_parts = Vec::new();

    if let Some(gh) = github {
        let mut author_commits: HashMap<&str, usize> = HashMap::new();
        for commit in &gh.commits {
            *author_commits.entry(&commit.author).or_insert(0) += 1;
        }
        let num_authors = author_commits.len();
        let total_commits = gh.commits.len();
        context_parts.push(format!(
            "{} contributors made {} commits",
            num_authors, total_commits
        ));
    }

    if let Some(ai_data) = ai {
        context_parts.push(format!(
            "{} AI sessions used, {:.1}:1 AI:Human turn ratio",
            ai_data.session_count,
            if ai_data.total_turns.human > 0 {
                ai_data.total_turns.assistant as f64 / ai_data.total_turns.human as f64
            } else {
                0.0
            }
        ));
    }

    if let Some(m) = manifest {
        if !m.team.learning_goals.is_empty() {
            context_parts.push(format!(
                "Learning goals: {}",
                m.team.learning_goals.join("; ")
            ));
        }
    }

    ReflectionQuestion {
        principle: Principle::DeveloperGrowth,
        data_context: if context_parts.is_empty() {
            "No data available for this period.".to_string()
        } else {
            context_parts.join(". ") + "."
        },
        question: "Are team members learning new skills through this work, \
            or becoming more dependent on AI to accomplish tasks they could \
            previously do themselves?"
            .to_string(),
    }
}

fn build_who_benefits_reflection(
    github: Option<&RepoSummary>,
    manifest: Option<&Manifest>,
) -> ReflectionQuestion {
    let mut context = if let Some(gh) = github {
        let merged: Vec<_> = gh
            .pull_requests
            .iter()
            .filter(|pr| pr.merged_at.is_some())
            .collect();
        if merged.is_empty() {
            "No PRs merged this period.".to_string()
        } else {
            let titles: Vec<_> = merged.iter().take(5).map(|pr| pr.title.clone()).collect();
            let suffix = if merged.len() > 5 {
                format!(" (and {} more)", merged.len() - 5)
            } else {
                String::new()
            };
            format!("Merged PRs: {}{}", titles.join(", "), suffix)
        }
    } else {
        "No GitHub data available.".to_string()
    };

    if let Some(m) = manifest {
        if !m.project.beneficiaries.is_empty() {
            let names: Vec<_> = m.project.beneficiaries.iter().map(|b| b.name.as_str()).collect();
            context = format!(
                "{}. Stated beneficiaries: {}",
                context.trim_end_matches('.'),
                names.join(", ")
            );
        }
    }

    ReflectionQuestion {
        principle: Principle::EquityOfBenefit,
        data_context: context,
        question: "Who benefits from the work shipped this period? \
            Does it serve users broadly, or does it primarily benefit \
            high-value accounts, internal stakeholders, or metrics \
            that don't map to real human need?"
            .to_string(),
    }
}

fn build_transparency_reflection(ai: Option<&AiUsageSummary>) -> ReflectionQuestion {
    let context = if let Some(ai_data) = ai {
        format!(
            "{} AI sessions, {} file write operations, {} file edits.",
            ai_data.session_count,
            ai_data.tools_used.get("Write").unwrap_or(&0),
            ai_data.tools_used.get("Edit").unwrap_or(&0),
        )
    } else {
        "No AI usage data available.".to_string()
    };

    ReflectionQuestion {
        principle: Principle::Transparency,
        data_context: context,
        question: "Would we be comfortable if a stakeholder asked us exactly \
            how AI was used in this work? Is AI involvement disclosed in PRs, \
            commit messages, or documentation?"
            .to_string(),
    }
}

fn build_proportionality_reflection(
    github: Option<&RepoSummary>,
    ai: Option<&AiUsageSummary>,
    manifest: Option<&Manifest>,
) -> ReflectionQuestion {
    let mut context_parts = Vec::new();

    if let Some(ai_data) = ai {
        let output_tokens = ai_data.total_tokens.output;
        let energy_config = crate::ethics::manifest::EnergyConfig::default();
        let est = crate::analysis::energy::estimate_total_energy(
            &ai_data.sessions,
            &energy_config,
        );
        context_parts.push(format!(
            "{:.1}K output tokens consumed across {} sessions (estimated ~{:.0} Wh energy)",
            output_tokens as f64 / 1_000.0,
            ai_data.session_count,
            est.total_wh
        ));
    }

    if let Some(gh) = github {
        let merged = gh
            .pull_requests
            .iter()
            .filter(|pr| pr.merged_at.is_some())
            .count();
        context_parts.push(format!("{} PRs merged", merged));
    }

    if let Some(m) = manifest {
        if !m.monthly_review.revenue_impact.is_empty() {
            context_parts.push(format!(
                "Revenue impact: \"{}\"",
                m.monthly_review.revenue_impact
            ));
        }
    }

    ReflectionQuestion {
        principle: Principle::EnvironmentalCost,
        data_context: if context_parts.is_empty() {
            "No data available.".to_string()
        } else {
            context_parts.join(". ") + "."
        },
        question: "Is the AI compute consumed proportionate to the value \
            delivered? Are there tasks where AI was used that could have \
            been done just as well without it?"
            .to_string(),
    }
}

fn build_babel_or_jerusalem_reflection(
    github: Option<&RepoSummary>,
    ai: Option<&AiUsageSummary>,
) -> ReflectionQuestion {
    let mut context_parts = Vec::new();

    if let Some(gh) = github {
        let mut authors: HashMap<&str, usize> = HashMap::new();
        for commit in &gh.commits {
            *authors.entry(&commit.author).or_insert(0) += 1;
        }
        context_parts.push(format!("{} contributors active", authors.len()));
    }

    if let Some(ai_data) = ai {
        context_parts.push(format!(
            "AI used for {} tool operations ({} Bash, {} Write, {} Edit)",
            ai_data.tools_used.values().sum::<u64>(),
            ai_data.tools_used.get("Bash").unwrap_or(&0),
            ai_data.tools_used.get("Write").unwrap_or(&0),
            ai_data.tools_used.get("Edit").unwrap_or(&0),
        ));
    }

    ReflectionQuestion {
        principle: Principle::HumanAgency,
        data_context: if context_parts.is_empty() {
            "No data available.".to_string()
        } else {
            context_parts.join(". ") + "."
        },
        question: "Is this work building Jerusalem — shared responsibility, \
            piece by piece, with everyone contributing their section of the wall? \
            Or is it building Babel — impressive but concentrated, optimizing for \
            output over human connection and growth?"
            .to_string(),
    }
}

fn build_security_reflection(ai: Option<&AiUsageSummary>) -> ReflectionQuestion {
    let context = if let Some(ai_data) = ai {
        let bash_count = ai_data.all_bash_commands.len();
        let sensitive_files = ai_data
            .sessions
            .iter()
            .flat_map(|s| &s.files_touched)
            .filter(|f| {
                let filename = f.path.rsplit('/').next().unwrap_or(&f.path).to_lowercase();
                [".env", ".pem", ".key", "secret", "credential", "id_rsa"]
                    .iter()
                    .any(|pat| filename.contains(pat))
            })
            .count();

        let mut parts = Vec::new();
        parts.push(format!("{} bash commands executed by AI", bash_count));
        if sensitive_files > 0 {
            parts.push(format!("{} sensitive file(s) accessed", sensitive_files));
        }
        parts.join(". ") + "."
    } else {
        "No AI usage data available.".to_string()
    };

    ReflectionQuestion {
        principle: Principle::Security,
        data_context: context,
        question: "Are there adequate guardrails on what AI tools can access and \
            execute in your environment? Has anyone reviewed the bash commands \
            AI ran for security concerns?"
            .to_string(),
    }
}

fn build_agent_autonomy_reflection(ai: &AiUsageSummary) -> ReflectionQuestion {
    let actions = &ai.agent_actions_summary;
    let context = format!(
        "{} total agent actions: {} messages sent, {} emails sent, \
        {} meetings scheduled. {} actions required approval, \
        {} were approved, {} denied.",
        actions.total_actions,
        actions.messages_sent,
        actions.emails_sent,
        actions.meetings_scheduled,
        actions.approvals_requested,
        actions.approvals_granted,
        actions.approvals_denied,
    );

    ReflectionQuestion {
        principle: Principle::Transparency,
        data_context: context,
        question: "When AI sends messages or emails on your behalf, \
            do the recipients know they're interacting with an AI? \
            Would your relationships change if people discovered \
            that an AI was handling your communications? \
            Are you losing the ability to communicate effectively \
            without AI assistance?"
            .to_string(),
    }
}

fn build_revenue_reflection(manifest: &Manifest) -> ReflectionQuestion {
    let mut context_parts = Vec::new();
    context_parts.push(format!(
        "Revenue impact: \"{}\"",
        manifest.monthly_review.revenue_impact
    ));

    if !manifest.monthly_review.value_delivered.is_empty() {
        context_parts.push(format!(
            "Value delivered: \"{}\"",
            manifest.monthly_review.value_delivered
        ));
    }

    if !manifest.project.mission.is_empty() {
        context_parts.push(format!("Mission: \"{}\"", manifest.project.mission));
    }

    ReflectionQuestion {
        principle: Principle::EquityOfBenefit,
        data_context: context_parts.join(". ") + ".",
        question: "Is the revenue generated aligned with the project's mission? \
            Does the business model create value for beneficiaries, or does it \
            extract value from them? Would this revenue exist without AI assistance, \
            and if so, who would have earned it?"
            .to_string(),
    }
}
