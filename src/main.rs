use clap::{Parser, Subcommand};
use conscience::ai_tools;
use conscience::analysis;
use conscience::config::Config;
use conscience::dashboard;
use conscience::ethics;
use conscience::ingest;
use conscience::report;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "conscience",
    about = "Evaluate the impact of AI-assisted work",
    version,
    long_about = "Conscience measures velocity, quality, business value, developer experience,\nand ethical implications of AI-assisted development work."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest data from a source
    Ingest {
        #[command(subcommand)]
        source: IngestSource,
    },
    /// Display a summary report
    Report {
        #[command(subcommand)]
        source: ReportSource,
    },
    /// Analyze who is writing code vs. who is operating AI tools
    Authorship {
        /// GitHub repository (owner/repo)
        #[arg(long)]
        repo: String,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory to match AI sessions
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Cross-project ethical analysis across all Claude Code projects
    ExamineAll {
        /// Number of days to look back for GitHub data
        #[arg(long, default_value = "30")]
        days: u32,
        /// Output as JSON instead of formatted text
        #[arg(long)]
        json: bool,
    },
    /// Push analysis results to a dashboard server
    Push {
        /// GitHub repository (owner/repo)
        #[arg(long)]
        repo: Option<String>,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory
        #[arg(long)]
        project: Option<PathBuf>,
        /// Dashboard endpoint URL (overrides config)
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Generate reflection questions for a team retrospective
    Reflect {
        /// GitHub repository (owner/repo) to enrich questions with activity data
        #[arg(long)]
        repo: Option<String>,
        /// Number of days to look back for GitHub data
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory to filter AI logs
        #[arg(long)]
        project: Option<PathBuf>,
        /// Answer each question at a prompt, then see a session summary
        #[arg(long, short)]
        interactive: bool,
        /// Output as JSON instead of formatted text
        #[arg(long)]
        json: bool,
    },
    /// Analyze attention patterns across projects
    Attention {
        /// Number of days to look back
        #[arg(long, default_value = "7")]
        days: u32,
        /// Filter to a specific project directory for AI logs
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Write an HTML timeline visualization to this path
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Ethical analysis: signals, scorecard, and reflection questions
    Examine {
        /// GitHub repository (owner/repo)
        #[arg(long)]
        repo: Option<String>,
        /// Number of days to look back for GitHub data
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory to filter AI logs
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON instead of formatted text
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum IngestSource {
    /// Ingest from GitHub (commits, PRs, reviews)
    Github {
        /// Repository in owner/repo format
        #[arg(long)]
        repo: String,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
    },
    /// Ingest from Claude Code session logs
    ClaudeCode {
        /// Filter to a specific project directory
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ReportSource {
    /// Report on GitHub activity
    Github {
        /// GitHub repository (owner/repo)
        #[arg(long)]
        repo: String,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
    },
    /// Report on AI tool usage
    Ai {
        /// Filter to a specific AI tool
        #[arg(long, value_parser = parse_ai_tool)]
        tool: Option<String>,
        /// Filter to a specific project directory
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

fn parse_ai_tool(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "claude-code" | "claude" | "claudecode" => Ok("claude-code".to_string()),
        "copilot" | "github-copilot" => Ok("copilot".to_string()),
        "cursor" => Ok("cursor".to_string()),
        "codex" | "openai-codex" => Ok("codex".to_string()),
        "windsurf" => Ok("windsurf".to_string()),
        "openclaw" | "open-claw" => Ok("openclaw".to_string()),
        "nanoclaw" | "nano-claw" => Ok("nanoclaw".to_string()),
        other => Err(format!(
            "Unknown AI tool '{}'. Supported: claude-code, copilot, cursor, codex, windsurf, openclaw, nanoclaw",
            other
        )),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Ingest { source } => match source {
            IngestSource::Github { repo, days } => run_github_ingest(&repo, days).await,
            IngestSource::ClaudeCode { project } => run_claude_code_ingest(project.as_deref()),
        },
        Commands::Report { source } => match source {
            ReportSource::Github { repo, days } => run_github_report(&repo, days).await,
            ReportSource::Ai { tool, project } => run_ai_report(tool.as_deref(), project.as_deref()),
        },
        Commands::Authorship {
            repo,
            days,
            project,
            json,
        } => run_authorship(&repo, days, project.as_deref(), json).await,
        Commands::Attention { days, project, json, html } => {
            run_attention(days, project.as_deref(), json, html.as_deref()).await
        }
        Commands::ExamineAll { days, json } => run_examine_all(days, json).await,
        Commands::Push {
            repo,
            days,
            project,
            endpoint,
        } => run_push(repo.as_deref(), days, project.as_deref(), endpoint.as_deref()).await,
        Commands::Examine {
            repo,
            days,
            project,
            json,
        } => run_examine(repo.as_deref(), days, project.as_deref(), json).await,
        Commands::Reflect {
            repo,
            days,
            project,
            interactive,
            json,
        } => run_reflect(repo.as_deref(), days, project.as_deref(), interactive, json).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run_github_ingest(repo: &str, days: u32) -> Result<(), Box<dyn std::error::Error>> {
    let summary = ingest::github::ingest_github(repo, days).await?;
    let json = serde_json::to_string_pretty(&summary)?;
    println!("{}", json);
    Ok(())
}

async fn run_github_report(repo: &str, days: u32) -> Result<(), Box<dyn std::error::Error>> {
    let summary = ingest::github::ingest_github(repo, days).await?;
    report::print_summary(&summary);
    Ok(())
}

fn run_claude_code_ingest(
    project: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = ingest::ai::ingest_claude_code(project)?;
    let json = serde_json::to_string_pretty(&summary)?;
    println!("{}", json);
    Ok(())
}

fn run_ai_report(
    tool: Option<&str>,
    project: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let show_claude = tool.is_none() || tool == Some("claude-code");

    if show_claude {
        let summary = ingest::ai::ingest_claude_code(project)?;
        ai_tools::report::print_ai_summary(&summary);
    }

    if let Some(t) = tool {
        if t != "claude-code" {
            eprintln!(
                "Parser for '{}' is not yet implemented. \
                Contributions welcome!",
                t
            );
        }
    }

    Ok(())
}

async fn run_authorship(
    repo: &str,
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let github_summary = ingest::github::ingest_github(repo, days).await?;
    let ai_summary = ingest::ai::ingest_claude_code(project)?;

    if ai_summary.session_count == 0 {
        eprintln!("No Claude Code sessions found. Authorship analysis requires AI session data.");
        std::process::exit(1);
    }

    let result = analysis::authorship::analyze_authorship(&github_summary, &ai_summary);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        analysis::authorship::print_authorship_analysis(&result);
    }

    Ok(())
}

async fn run_attention(
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
    _html_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = ingest::ai::ingest_claude_code(project)?;
    if summary.session_count == 0 {
        eprintln!("No Claude Code sessions found.");
        std::process::exit(1);
    }

    let cwd = std::env::current_dir()?;
    let manifest_dir = project.unwrap_or(&cwd);
    let manifest = ethics::manifest::Manifest::load(manifest_dir);
    let th = manifest
        .as_ref()
        .map(|m| m.thresholds.attention.clone())
        .unwrap_or_default();

    let tz = *chrono::Local::now().offset();
    let analysis = analysis::attention::analyze_attention(&summary.sessions, &th, days, tz);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
    } else {
        analysis::attention_report::print_attention_analysis(&analysis);
    }

    Ok(())
}

async fn run_push(
    repo: Option<&str>,
    days: u32,
    project: Option<&std::path::Path>,
    endpoint_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    let endpoint = endpoint_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("CONSCIENCE_DASHBOARD_URL").ok())
        .or(config.dashboard.endpoint)
        .ok_or("No dashboard endpoint. Set --endpoint, CONSCIENCE_DASHBOARD_URL, or configure in ~/.conscience/config.toml")?;

    let api_key = std::env::var("CONSCIENCE_DASHBOARD_API_KEY")
        .ok()
        .or(config.dashboard.api_key);

    let cwd = std::env::current_dir()?;
    let manifest_dir = project.unwrap_or(&cwd);
    let manifest = ethics::manifest::Manifest::load(manifest_dir);

    let github_summary = if let Some(r) = repo {
        Some(ingest::github::ingest_github(r, days).await?)
    } else {
        None
    };

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(project)?;
        if summary.session_count > 0 {
            Some(summary)
        } else {
            None
        }
    };

    let analysis = ethics::analyze(
        github_summary.as_ref(),
        ai_summary.as_ref(),
        manifest.as_ref(),
    );

    let project_name = manifest
        .as_ref()
        .map(|m| m.project.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            manifest_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let payload = dashboard::models::DashboardPayload::new(
        project_name,
        repo.map(|s| s.to_string()),
        project.map(|p| p.to_string_lossy().to_string()),
        analysis,
        ai_summary.as_ref().map(|s| s.session_count),
        ai_summary.as_ref().map(|s| s.total_tokens.output),
        days,
    );

    dashboard::push::push_analysis(&endpoint, &payload, api_key.as_deref()).await?;

    Ok(())
}

async fn run_examine_all(
    days: u32,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let analysis = ethics::multi::analyze_all_projects(days).await?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
    } else {
        ethics::report::print_multi_project_analysis(&analysis);
    }

    Ok(())
}

async fn run_reflect(
    repo: Option<&str>,
    days: u32,
    project: Option<&std::path::Path>,
    interactive: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let manifest_dir = project.unwrap_or(&cwd);
    let manifest = ethics::manifest::Manifest::load(manifest_dir);

    let github_summary = if let Some(r) = repo {
        Some(ingest::github::ingest_github(r, days).await?)
    } else {
        None
    };

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(project)?;
        if summary.session_count > 0 {
            Some(summary)
        } else {
            None
        }
    };

    // Unlike examine, reflect is useful with no data at all — the
    // questions stand on their own, data only enriches them.
    if github_summary.is_none() && ai_summary.is_none() {
        eprintln!(
            "No GitHub or AI session data found; questions will lack data context. \
            Provide --repo and/or --project to enrich them."
        );
    }

    let reflections = ethics::reflection::generate_reflections(
        github_summary.as_ref(),
        ai_summary.as_ref(),
        manifest.as_ref(),
    );

    if interactive {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        println!();
        println!("  Reflection Session");
        println!("  For team discussion \u{2014} not automated judgment.");
        let responses = ethics::session::run_session(&reflections, stdin.lock(), stdout.lock());

        if json_output {
            println!("{}", serde_json::to_string_pretty(&responses)?);
        } else {
            println!();
            print!("{}", ethics::session::render_summary(&responses));
        }
    } else if json_output {
        println!("{}", serde_json::to_string_pretty(&reflections)?);
    } else {
        println!();
        print!(
            "{}",
            ethics::report::render_reflection_session(&reflections)
        );
    }

    Ok(())
}

async fn run_examine(
    repo: Option<&str>,
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load conscience.yaml if present
    let cwd = std::env::current_dir()?;
    let manifest_dir = project.unwrap_or(&cwd);
    let manifest = ethics::manifest::Manifest::load(manifest_dir);
    if manifest.is_some() {
        eprintln!("Loaded conscience.yaml from {}", manifest_dir.display());
    }

    let github_summary = if let Some(r) = repo {
        Some(ingest::github::ingest_github(r, days).await?)
    } else {
        None
    };

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(project)?;
        if summary.session_count > 0 {
            Some(summary)
        } else {
            None
        }
    };

    if github_summary.is_none() && ai_summary.is_none() {
        eprintln!("No data sources available. Provide --repo and/or --project.");
        std::process::exit(1);
    }

    let analysis = ethics::analyze(
        github_summary.as_ref(),
        ai_summary.as_ref(),
        manifest.as_ref(),
    );

    if json_output {
        let output = serde_json::to_string_pretty(&analysis)?;
        println!("{}", output);
    } else {
        ethics::report::print_ethical_analysis(&analysis);
    }

    Ok(())
}
