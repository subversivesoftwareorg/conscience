use clap::{Parser, Subcommand};
use conscience::ai_tools;
use conscience::analysis;
use conscience::config::Config;
use conscience::dashboard;
use conscience::ethics;
use conscience::ingest;
use conscience::interval::Interval;
use conscience::project::{self, ProjectScope};
use conscience::report;
use std::path::{Path, PathBuf};

/// Scope for commands that operate on one project: `--project` if given,
/// otherwise the current directory. Fails if the path does not exist.
fn project_scope(project: Option<&Path>) -> Result<ProjectScope, Box<dyn std::error::Error>> {
    Ok(project::resolve_project(project)?)
}

/// Scope for commands that default to every project on this machine:
/// only restricted when `--project` is given explicitly.
fn optional_scope(
    project: Option<&Path>,
) -> Result<Option<ProjectScope>, Box<dyn std::error::Error>> {
    match project {
        Some(p) => Ok(Some(project::resolve_project(Some(p))?)),
        None => Ok(None),
    }
}

/// Manifest for a cross-project command: from the explicit project if given,
/// otherwise from the current directory (thresholds still apply to "all").
fn manifest_for(scope: Option<&ProjectScope>) -> Option<ethics::manifest::Manifest> {
    match scope {
        Some(s) => s.manifest.clone(),
        None => std::env::current_dir()
            .ok()
            .and_then(|cwd| ethics::manifest::Manifest::load(&cwd)),
    }
}

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
        /// Save session answers to a JSON file (default: .conscience/reflections/YYYY-MM-DD.json)
        #[arg(long)]
        save: Option<Option<PathBuf>>,
        /// Output as JSON instead of formatted text
        #[arg(long)]
        json: bool,
    },
    /// Aggregate saved reflection sessions into a team retrospective view
    Retro {
        /// Directory containing saved reflection JSON files (default: .conscience/reflections/)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate a weekly digest summarizing ethical signal trends across repos
    Digest {
        /// Number of days to look back
        #[arg(long, default_value = "7")]
        days: u32,
        /// Write digest to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
        /// Output as JSON instead of Markdown
        #[arg(long)]
        json: bool,
    },
    /// Retrospective on token consumption — where did the budget go?
    RetroTokens {
        /// Hours to look back
        #[arg(long, default_value = "4")]
        hours: u32,
        /// Filter to a specific project
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON
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
    /// Check which integrations are configured and show setup directions
    Setup,
    /// Ethical analysis scoped to a single pull request
    Evaluate {
        /// GitHub PR URL (https://github.com/owner/repo/pull/N) or short form (owner/repo#N)
        #[arg(long)]
        pr: String,
        /// Project directory to enrich with AI session context
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        /// Filter to a specific AI tool (claude-code, copilot, cursor, codex, windsurf, openclaw, nanoclaw)
        #[arg(long, value_parser = parse_ai_tool)]
        tool: Option<String>,
        /// Filter to a specific project directory
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Report on estimated energy consumption of AI usage
    Energy {
        /// Filter to a specific project directory
        #[arg(long)]
        project: Option<PathBuf>,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
            ReportSource::Energy { project, days, json } => run_energy_report(project.as_deref(), days, json),
        },
        Commands::Authorship {
            repo,
            days,
            project,
            json,
        } => run_authorship(&repo, days, project.as_deref(), json).await,
        Commands::Digest { days, output, json } => run_digest(days, output.as_deref(), json).await,
        Commands::RetroTokens { hours, project, json } => run_retro_tokens(hours, project.as_deref(), json),
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
        Commands::Setup => run_setup(),
        Commands::Evaluate { pr, project, json } => {
            run_evaluate(&pr, project.as_deref(), json).await
        }
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
            save,
            json,
        } => run_reflect(repo.as_deref(), days, project.as_deref(), interactive, save, json).await,
        Commands::Retro { dir, days, json } => run_retro(dir.as_deref(), days, json),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_setup() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let ok = "\x1b[32m\u{2713}\x1b[0m";
    let fail = "\x1b[31m\u{2717}\x1b[0m";

    println!();
    println!("  Conscience Setup");
    println!();

    // 1. GitHub authentication
    let gh_installed = Command::new("gh").arg("--version").output().is_ok();
    if !gh_installed {
        println!("  {} GitHub CLI (gh not found)", fail);
        println!("    \u{2192} Install: https://cli.github.com/");
        println!("    \u{2192} Then run: gh auth login");
    } else {
        match Command::new("gh").args(["auth", "token"]).output() {
            Ok(output) if output.status.success() => {
                println!("  {} GitHub authentication (gh CLI authenticated)", ok);
            }
            _ => {
                let has_env = std::env::var("CONSCIENCE_GITHUB_TOKEN")
                    .map(|t| !t.is_empty())
                    .unwrap_or(false);
                let has_config = Config::load().github.token.is_some();
                if has_env {
                    println!("  {} GitHub authentication (CONSCIENCE_GITHUB_TOKEN set)", ok);
                } else if has_config {
                    println!("  {} GitHub authentication (~/.conscience/config.toml)", ok);
                } else {
                    println!("  {} GitHub authentication (not configured)", fail);
                    println!("    \u{2192} Easiest: gh auth login");
                    println!("    \u{2192} Or set CONSCIENCE_GITHUB_TOKEN environment variable");
                    println!("    \u{2192} Or add token to ~/.conscience/config.toml");
                }
            }
        }
    }

    // 2. conscience.yaml
    let cwd = std::env::current_dir().unwrap_or_default();
    let has_manifest = cwd.join("conscience.yaml").exists() || cwd.join(".conscience.yaml").exists();
    if has_manifest {
        println!("  {} conscience.yaml (found in current directory)", ok);
    } else {
        println!("  {} conscience.yaml (not found in current directory)", fail);
        println!("    \u{2192} Create conscience.yaml with your project name and mission");
        println!("    \u{2192} See: https://github.com/subversivesoftwareorg/conscience#configuration-conscienceyaml");
    }

    // 3. Claude Code logs
    let claude_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("projects");
    if claude_dir.exists() {
        let session_count = std::fs::read_dir(&claude_dir)
            .into_iter()
            .flatten()
            .flat_map(|e| {
                let p = e.ok()?.path();
                if p.is_dir() {
                    Some(
                        std::fs::read_dir(&p)
                            .into_iter()
                            .flatten()
                            .flatten()
                            .filter(|f| {
                                f.path().extension().is_some_and(|e| e == "jsonl")
                            })
                            .count(),
                    )
                } else {
                    None
                }
            })
            .sum::<usize>();
        println!(
            "  {} Claude Code logs ({} sessions found in ~/.claude/projects/)",
            ok, session_count
        );
    } else {
        println!("  {} Claude Code logs (no ~/.claude/projects/ directory)", fail);
        println!("    \u{2192} Use Claude Code to generate session logs automatically");
        println!("    \u{2192} Logs appear after your first Claude Code session");
    }

    // 3b. Other AI tools
    use crate::ai_tools::parser::AiToolParser;
    let other_tools: Vec<Box<dyn AiToolParser>> = vec![
        Box::new(ai_tools::copilot::CopilotParser),
        Box::new(ai_tools::cursor::CursorParser),
        Box::new(ai_tools::codex::CodexParser::new()),
        Box::new(ai_tools::windsurf::WindsurfParser),
    ];
    for tool in &other_tools {
        if tool.detect() {
            println!(
                "  {} {} (detected at {} \u{2014} parser not yet implemented)",
                ok, tool.tool_name(), tool.data_path()
            );
            println!("    \u{2192} Help us build the parser: share sample log data in the GitHub issue");
        }
    }

    // 4. Dashboard
    let config = Config::load();
    let has_dashboard = std::env::var("CONSCIENCE_DASHBOARD_URL").is_ok()
        || config.dashboard.endpoint.is_some();
    if has_dashboard {
        let endpoint = std::env::var("CONSCIENCE_DASHBOARD_URL")
            .ok()
            .or(config.dashboard.endpoint)
            .unwrap_or_default();
        println!("  {} Dashboard endpoint ({})", ok, endpoint);
    } else {
        println!("  {} Dashboard endpoint (not configured \u{2014} optional)", fail);
        println!("    \u{2192} Set CONSCIENCE_DASHBOARD_URL or add [dashboard] to ~/.conscience/config.toml");
    }

    // 5. GitHub Actions workflow
    let has_workflow = std::path::Path::new(".github/workflows/conscience.yml").exists();
    if has_workflow {
        println!("  {} GitHub Actions workflow (.github/workflows/conscience.yml)", ok);
    } else {
        println!("  {} GitHub Actions workflow (not found \u{2014} optional)", fail);
        println!("    \u{2192} Copy conscience.yml from the conscience repo to .github/workflows/");
        println!("    \u{2192} Runs ethical analysis on PRs and weekly");
    }

    println!();

    Ok(())
}

async fn run_github_ingest(repo: &str, days: u32) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let summary = ingest::github::ingest_github(repo, &interval).await?;
    let json = serde_json::to_string_pretty(&summary)?;
    println!("{}", json);
    Ok(())
}

async fn run_github_report(repo: &str, days: u32) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let summary = ingest::github::ingest_github(repo, &interval).await?;
    report::print_summary(&summary);
    Ok(())
}

fn run_claude_code_ingest(
    project: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = optional_scope(project)?;
    // Raw ingest is deliberately all-time; it is a debugging view of the logs.
    let summary = ingest::ai::ingest_claude_code(scope.as_ref(), None)?;
    let json = serde_json::to_string_pretty(&summary)?;
    println!("{}", json);
    Ok(())
}

fn run_ai_report(
    tool: Option<&str>,
    project: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let show_claude = tool.is_none() || tool == Some("claude-code");
    let show_codex = tool.is_none() || tool == Some("codex");
    let scope = optional_scope(project)?;

    if show_claude {
        let summary = ingest::ai::ingest_claude_code(scope.as_ref(), None)?;
        ai_tools::report::print_ai_summary(&summary);
    }

    if show_codex {
        use crate::ai_tools::parser::AiToolParser as _;
        let parser = ai_tools::codex::CodexParser::new();
        if parser.detect() {
            match parser.parse(scope.as_ref()) {
                Ok(summary) if summary.session_count > 0 => {
                    ai_tools::report::print_ai_summary(&summary);
                }
                Ok(_) => {}
                Err(e) => eprintln!("  Warning: Codex parser error: {}", e),
            }
        }
    }

    if let Some(t) = tool {
        if !matches!(t, "claude-code" | "codex") {
            eprintln!(
                "Parser for '{}' is not yet implemented. \
                Contributions welcome!",
                t
            );
        }
    }

    Ok(())
}

fn run_energy_report(
    project: Option<&std::path::Path>,
    days: u32,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = optional_scope(project)?;
    let interval = Interval::last_days(days);
    let summary = ingest::ai::ingest_claude_code(scope.as_ref(), Some(&interval))?;
    if summary.session_count == 0 {
        eprintln!("No Claude Code sessions found in the last {} days.", days);
        std::process::exit(1);
    }

    let manifest = manifest_for(scope.as_ref());
    let config = manifest
        .as_ref()
        .map(|m| m.thresholds.energy.clone())
        .unwrap_or_default();

    let mut estimate = analysis::energy::estimate_total_energy(&summary.sessions, &config);
    estimate.period_days = days;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&estimate)?);
    } else {
        analysis::energy_report::print_energy_report(&estimate);
    }

    Ok(())
}

async fn run_authorship(
    repo: &str,
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let github_summary = ingest::github::ingest_github(repo, &interval).await?;
    let scope = optional_scope(project)?;
    let ai_summary = ingest::ai::ingest_claude_code(scope.as_ref(), Some(&interval))?;

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
    html_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = optional_scope(project)?;
    let interval = Interval::last_days(days);
    let summary = ingest::ai::ingest_claude_code(scope.as_ref(), Some(&interval))?;
    if summary.session_count == 0 {
        eprintln!("No Claude Code sessions found in the last {} days.", days);
        std::process::exit(1);
    }

    let manifest = manifest_for(scope.as_ref());
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

    if let Some(path) = html_path {
        let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let tps = analysis::attention::collect_touchpoints(&summary.sessions, &th, since);
        let html = analysis::attention_html::render_html_timeline(&analysis, &tps, tz);
        std::fs::write(path, &html)?;
        eprintln!("Timeline written to {}", path.display());
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

    let scope = project_scope(project)?;
    let manifest = scope.manifest.clone();
    let interval = Interval::last_days(days);

    let github_summary = if let Some(r) = repo {
        Some(ingest::github::ingest_github(r, &interval).await?)
    } else {
        None
    };

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(Some(&scope), Some(&interval))?;
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

    let project_name = scope.display_name();

    let payload = dashboard::models::DashboardPayload::new(
        project_name,
        repo.map(|s| s.to_string()),
        Some(scope.root.to_string_lossy().to_string()),
        analysis,
        ai_summary.as_ref().map(|s| s.session_count),
        ai_summary.as_ref().map(|s| s.total_tokens.output),
        ai_summary.as_ref().map(|s| s.undated_sessions).unwrap_or(0),
        &interval,
    );

    dashboard::push::push_analysis(&endpoint, &payload, api_key.as_deref()).await?;

    Ok(())
}

fn run_retro_tokens(
    hours: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = optional_scope(project)?;
    let interval = Interval::last_hours(hours);
    let summary = ingest::ai::ingest_claude_code(scope.as_ref(), Some(&interval))?;
    let recent = summary.sessions;

    if recent.is_empty() {
        eprintln!("No sessions active in the last {} hours.", hours);
        std::process::exit(1);
    }

    eprintln!("{} session(s) active in the last {} hours", recent.len(), hours);

    let retro = analysis::session_retro::analyze_token_retro(&recent);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&retro)?);
        return Ok(());
    }

    println!();
    println!(
        "  Conscience \u{2014} Token Retrospective (last {} hours)",
        hours
    );
    println!(
        "  {} sessions | {}K total tokens | ~{:.0} Wh estimated energy",
        retro.session_count,
        retro.total_tokens / 1_000,
        retro.total_energy_wh,
    );
    println!(
        "  Cache efficiency: {:.0}%",
        retro.cache_efficiency,
    );
    if retro.total_agent_dispatches > 0 || retro.total_skill_invocations > 0 {
        println!(
            "  Orchestration: {} agent dispatches, {} skill invocations",
            retro.total_agent_dispatches, retro.total_skill_invocations,
        );
    }
    println!();

    // Per-session table
    println!("  Per-Session Breakdown (ranked by token consumption)");
    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            comfy_table::Cell::new("Session").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Project").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Model").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Tokens").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("In%").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Out%").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Cache%").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Agents").fg(comfy_table::Color::Cyan),
            comfy_table::Cell::new("Skills").fg(comfy_table::Color::Cyan),
        ]);

    for p in &retro.per_session {
        let sid = p.session_id.chars().take(8).collect::<String>();
        let tokens = if p.total_tokens >= 1_000_000 {
            format!("{:.1}M", p.total_tokens as f64 / 1_000_000.0)
        } else {
            format!("{}K", p.total_tokens / 1_000)
        };
        table.add_row(vec![
            comfy_table::Cell::new(&sid),
            comfy_table::Cell::new(&p.project),
            comfy_table::Cell::new(p.model.chars().take(20).collect::<String>()),
            comfy_table::Cell::new(&tokens),
            comfy_table::Cell::new(format!("{:.0}", p.input_pct)),
            comfy_table::Cell::new(format!("{:.0}", p.output_pct)),
            comfy_table::Cell::new(format!("{:.0}", p.cache_efficiency)),
            comfy_table::Cell::new(p.agent_count.to_string()),
            comfy_table::Cell::new(p.skill_count.to_string()),
        ]);
    }
    println!("{}", table);

    // Agent details if any
    let sessions_with_agents: Vec<_> = retro.per_session.iter().filter(|p| p.agent_count > 0).collect();
    if !sessions_with_agents.is_empty() {
        println!();
        println!("  Agent Dispatches");
        for p in sessions_with_agents {
            println!(
                "    {} ({}) \u{2014} {} agents:",
                p.session_id.chars().take(8).collect::<String>(),
                p.project,
                p.agent_count,
            );
            for agent in &p.agents {
                println!("      \u{2022} {}", agent);
            }
        }
    }

    // Skill details if any
    let sessions_with_skills: Vec<_> = retro.per_session.iter().filter(|p| p.skill_count > 0).collect();
    if !sessions_with_skills.is_empty() {
        println!();
        println!("  Skill Invocations");
        for p in sessions_with_skills {
            let unique_skills: Vec<_> = {
                let mut s = p.skills.clone();
                s.sort();
                s.dedup();
                s
            };
            println!(
                "    {} ({}) \u{2014} {}",
                p.session_id.chars().take(8).collect::<String>(),
                p.project,
                unique_skills.join(", "),
            );
        }
    }

    // Top tools
    if !retro.tool_summary.is_empty() {
        println!();
        println!("  Top Tools (across all sessions)");
        for (tool, count) in retro.tool_summary.iter().take(8) {
            println!("    {:>5}x  {}", count, tool);
        }
    }

    // Diagnoses
    if !retro.diagnoses.is_empty() {
        println!();
        println!("  Diagnosis");
        for d in &retro.diagnoses {
            println!("    \u{2022} {}", d);
        }
    }

    println!();

    Ok(())
}

async fn run_digest(
    days: u32,
    output: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let analysis = ethics::multi::analyze_all_projects(&interval).await?;

    if analysis.total_projects == 0 {
        eprintln!("No projects found.");
        std::process::exit(1);
    }

    if json_output {
        let json = serde_json::to_string_pretty(&analysis)?;
        if let Some(path) = output {
            std::fs::write(path, &json)?;
            eprintln!("Digest written to {}", path.display());
        } else {
            println!("{}", json);
        }
        return Ok(());
    }

    let mut md = String::new();

    use std::fmt::Write;
    let _ = writeln!(md, "# Conscience Weekly Digest");
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "**Period:** {} days ending {} | **Projects:** {} | **Sessions:** {} | **Output tokens:** {}K",
        days,
        chrono::Utc::now().format("%Y-%m-%d"),
        analysis.total_projects,
        analysis.total_sessions,
        analysis.total_output_tokens / 1_000,
    );

    // Rough energy from total output tokens using large-tier default (1.5 Wh/1K output)
    let rough_energy_wh = analysis.total_output_tokens as f64 * 1.5 / 1000.0;
    let _ = writeln!(md, "**Estimated energy:** ~{:.0} Wh (~{:.1} hours of laptop use)", rough_energy_wh, rough_energy_wh / 60.0);
    let _ = writeln!(md);

    // Top Concerns
    let mut concerns: Vec<(&str, &ethics::models::Signal)> = Vec::new();
    for project in &analysis.projects {
        let name = project.project_name.as_deref().unwrap_or("unknown");
        for signal in &project.analysis.signals {
            if signal.severity >= ethics::models::Severity::Concern {
                concerns.push((name, signal));
            }
        }
    }
    for signal in &analysis.outlier_signals {
        if signal.severity >= ethics::models::Severity::Concern {
            concerns.push(("(cross-project)", signal));
        }
    }
    concerns.sort_by_key(|(_, s)| std::cmp::Reverse(s.severity));

    if concerns.is_empty() {
        let _ = writeln!(md, "## No Concerns");
        let _ = writeln!(md, "All projects are in healthy territory this period.");
    } else {
        let _ = writeln!(md, "## Top Concerns ({} signals)", concerns.len());
        let _ = writeln!(md);
        for (project, signal) in concerns.iter().take(10) {
            let _ = writeln!(
                md,
                "- **{}** {} — {} [{}]",
                signal.severity,
                project,
                signal.title,
                signal.principle.name()
            );
            let _ = writeln!(md, "  {}", signal.detail);
        }
        if concerns.len() > 10 {
            let _ = writeln!(md, "- ...and {} more", concerns.len() - 10);
        }
    }
    let _ = writeln!(md);

    // Healthy Patterns
    let healthy: Vec<(&str, &ethics::models::Signal)> = analysis
        .projects
        .iter()
        .flat_map(|p| {
            let name = p.project_name.as_deref().unwrap_or("unknown");
            p.analysis
                .signals
                .iter()
                .filter(|s| s.severity == ethics::models::Severity::Healthy)
                .map(move |s| (name, s))
        })
        .collect();

    if !healthy.is_empty() {
        let _ = writeln!(md, "## Healthy Patterns");
        let _ = writeln!(md);
        for (project, signal) in healthy.iter().take(5) {
            let _ = writeln!(md, "- **{}** — {} [{}]", project, signal.title, signal.principle.name());
        }
        if healthy.len() > 5 {
            let _ = writeln!(md, "- ...and {} more across projects", healthy.len() - 5);
        }
        let _ = writeln!(md);
    }

    // Per-project one-liners
    let _ = writeln!(md, "## Project Summary");
    let _ = writeln!(md);
    let _ = writeln!(md, "| Project | Sessions | Tokens | Signals | Worst |");
    let _ = writeln!(md, "|---------|----------|--------|---------|-------|");
    for p in &analysis.projects {
        let name = p.project_name.as_deref().unwrap_or("unknown");
        let worst = p
            .analysis
            .signals
            .iter()
            .map(|s| s.severity)
            .max()
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "—".to_string());
        let _ = writeln!(
            md,
            "| {} | {} | {}K | {} | {} |",
            name,
            p.session_count,
            p.total_output_tokens / 1_000,
            p.analysis.signals.len(),
            worst,
        );
    }
    let _ = writeln!(md);

    // Reflection
    let _ = writeln!(md, "## Reflection");
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "Looking at the past {} days across {} projects: is the AI compute ({} sessions, \
        ~{:.0} Wh estimated energy) proportionate to the value delivered? \
        Are the concerns above worth investigating, or are they noise?",
        days,
        analysis.total_projects,
        analysis.total_sessions,
        rough_energy_wh,
    );

    if let Some(path) = output {
        std::fs::write(path, &md)?;
        eprintln!("Digest written to {}", path.display());
    } else {
        print!("{}", md);
    }

    Ok(())
}

async fn run_examine_all(
    days: u32,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let analysis = ethics::multi::analyze_all_projects(&interval).await?;

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
    save: Option<Option<PathBuf>>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = project_scope(project)?;
    let manifest = scope.manifest.clone();
    let manifest_dir = scope.root.clone();
    let interval = Interval::last_days(days);

    let github_summary = if let Some(r) = repo {
        Some(ingest::github::ingest_github(r, &interval).await?)
    } else {
        None
    };

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(Some(&scope), Some(&interval))?;
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

        // --save: persist the session
        if let Some(path_opt) = save {
            let contributor = std::process::Command::new("git")
                .args(["config", "user.name"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

            let project_name = manifest
                .as_ref()
                .map(|m| m.project.name.clone())
                .filter(|n| !n.is_empty())
                .or_else(|| {
                    manifest_dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                });

            let session = ethics::session::ReflectionSession {
                timestamp: chrono::Utc::now().to_rfc3339(),
                contributor,
                project: project_name,
                responses,
            };

            let save_path = match path_opt {
                Some(p) => p,
                None => {
                    let dir = manifest_dir.join(".conscience").join("reflections");
                    std::fs::create_dir_all(&dir)?;
                    dir.join(format!("{}.json", chrono::Utc::now().format("%Y-%m-%d")))
                }
            };

            let json = serde_json::to_string_pretty(&session)?;
            std::fs::write(&save_path, &json)?;
            eprintln!("Session saved to {}", save_path.display());
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

async fn run_evaluate(
    pr_url: &str,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let github_summary = ingest::github::ingest_pr(pr_url).await?;

    let scope = project_scope(project)?;
    let manifest = scope.manifest.clone();

    // A PR's own lifetime is the interval: sessions that overlap the time
    // between it being opened and merged (or closed, or now if still open).
    let interval = Interval::between(github_summary.period_start, github_summary.period_end);

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(Some(&scope), Some(&interval))?;
        if summary.session_count > 0 {
            Some(summary)
        } else {
            None
        }
    };

    let analysis = ethics::analyze(
        Some(&github_summary),
        ai_summary.as_ref(),
        manifest.as_ref(),
    );

    let pr = &github_summary.pull_requests[0];
    if !json_output {
        println!();
        println!(
            "  Conscience \u{2014} PR #{}: \"{}\"",
            pr.number, pr.title
        );
        println!(
            "  by {} | +{}/\u{2212}{} | {} files | {} review comments",
            pr.author,
            pr.additions.unwrap_or(0),
            pr.deletions.unwrap_or(0),
            pr.changed_files.unwrap_or(0),
            pr.review_comments,
        );
    }

    if json_output {
        let output = serde_json::to_string_pretty(&analysis)?;
        println!("{}", output);
    } else {
        ethics::report::print_ethical_analysis(&analysis);
    }

    Ok(())
}

fn run_retro(
    dir: Option<&std::path::Path>,
    days: u32,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let cwd = std::env::current_dir()?;
    let reflections_dir = dir.unwrap_or_else(|| &cwd).join(".conscience").join("reflections");

    if !reflections_dir.exists() {
        eprintln!(
            "No reflections directory at {}. Run `conscience reflect -i --save` first.",
            reflections_dir.display()
        );
        std::process::exit(1);
    }

    let mut sessions = Vec::new();
    let mut out_of_range = 0usize;
    let mut undated = 0usize;
    for entry in std::fs::read_dir(&reflections_dir)?.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<ethics::session::ReflectionSession>(&content) {
                    Ok(session) => {
                        match session.timestamp.parse::<chrono::DateTime<chrono::Utc>>() {
                            Ok(at) if interval.contains(at) => sessions.push(session),
                            Ok(_) => out_of_range += 1,
                            Err(_) => undated += 1,
                        }
                    }
                    Err(e) => eprintln!("Warning: skipping {}: {}", path.display(), e),
                },
                Err(e) => eprintln!("Warning: could not read {}: {}", path.display(), e),
            }
        }
    }

    eprintln!(
        "Interval: {}; {} reflection session(s) in range, {} outside, {} undated",
        interval.label(),
        sessions.len(),
        out_of_range,
        undated
    );

    if sessions.is_empty() {
        eprintln!(
            "No reflection sessions in the last {} days in {}",
            days,
            reflections_dir.display()
        );
        std::process::exit(1);
    }

    let aggregate = ethics::session::aggregate_sessions(&sessions);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&aggregate)?);
    } else {
        println!();
        println!(
            "  Conscience \u{2014} Team Retrospective ({} sessions, {} contributors)",
            aggregate.session_count,
            aggregate.contributors.len()
        );
        println!("  Contributors: {}", aggregate.contributors.join(", "));
        println!();

        for pa in &aggregate.by_principle {
            println!("  {} [{}]", pa.question, pa.principle.name());
            if pa.answers.is_empty() && pa.skipped > 0 {
                println!("    (all {} respondents skipped)", pa.skipped);
            } else {
                for ca in &pa.answers {
                    let who = ca.contributor.as_deref().unwrap_or("(anonymous)");
                    println!("    {} \u{2014} {}", who, ca.answer);
                }
                if pa.skipped > 0 {
                    println!("    ({} skipped)", pa.skipped);
                }
            }
            println!();
        }
    }

    Ok(())
}

async fn run_examine(
    repo: Option<&str>,
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve the project and the interval once; both sources share them.
    let scope = project_scope(project)?;
    let manifest = scope.manifest.clone();
    if manifest.is_some() {
        eprintln!("Loaded conscience.yaml from {}", scope.root.display());
    }
    let interval = Interval::last_days(days);

    let github_summary = if let Some(r) = repo {
        Some(ingest::github::ingest_github(r, &interval).await?)
    } else {
        None
    };

    let ai_summary = {
        let summary = ingest::ai::ingest_claude_code(Some(&scope), Some(&interval))?;
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
