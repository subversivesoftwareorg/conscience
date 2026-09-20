use clap::{Parser, Subcommand};
use conscience::ai_tools;
use conscience::analysis;
use conscience::config::Config;
use conscience::dashboard;
use conscience::ethics;
use conscience::export::SnapshotExport;
use conscience::ingest;
use conscience::interval::Interval;
use conscience::pipeline;
use conscience::project::{self, ProjectScope};
use conscience::snapshot::Snapshot;
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

/// Scope for reports that default to the current project but accept `--all`.
fn report_scope(
    project: Option<&Path>,
    all: bool,
) -> Result<Option<ProjectScope>, Box<dyn std::error::Error>> {
    if all {
        Ok(None)
    } else {
        Ok(Some(project::resolve_project(project)?))
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
    /// Check which integrations are configured and show setup directions
    Setup,
    /// Ethical analysis of the current project: what was analyzed, what
    /// deserves attention, what is worth discussing. Writes a snapshot.
    Examine {
        /// GitHub repository (owner/repo). Default: github.repo in conscience.yaml, else the checkout's origin remote
        #[arg(long, conflicts_with_all = ["pr", "all", "no_github"])]
        repo: Option<String>,
        /// Do not collect GitHub data even if a repository can be detected
        #[arg(long, conflicts_with_all = ["pr", "all"])]
        no_github: bool,
        /// Analyze a single pull request (URL or owner/repo#N) over its own lifetime
        #[arg(long, conflicts_with = "all")]
        pr: Option<String>,
        /// Analyze every project with Claude Code data instead of one
        #[arg(long, conflicts_with = "project")]
        all: bool,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory (default: current directory)
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON instead of formatted text
        #[arg(long)]
        json: bool,
        /// Show every signal, the scorecard, and all reflection questions
        #[arg(long, conflicts_with_all = ["json", "all"])]
        full: bool,
        /// With --all: a Markdown digest. With --pr: the PR comment body, printed instead of posted
        #[arg(long, conflicts_with = "json")]
        markdown: bool,
        /// With --all --markdown: write the digest to a file
        #[arg(long, requires = "markdown")]
        output: Option<PathBuf>,
        /// With --pr: post the analysis as a comment on the pull request (edits its earlier comment if one exists)
        #[arg(long, requires = "pr")]
        comment: bool,
    },
    /// Diagnostic reports: github, ai, energy, tokens, authorship, attention
    Report {
        #[command(subcommand)]
        source: ReportSource,
    },
    /// Generate reflection questions for a team retrospective
    Reflect {
        /// GitHub repository (owner/repo). Default: github.repo in conscience.yaml, else the checkout's origin remote
        #[arg(long, conflicts_with = "no_github")]
        repo: Option<String>,
        /// Do not collect GitHub data even if a repository can be detected
        #[arg(long)]
        no_github: bool,
        /// Number of days to look back for GitHub data
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory to filter AI logs
        #[arg(long)]
        project: Option<PathBuf>,
        /// Answer each question at a prompt, then see a session summary
        #[arg(long, short)]
        interactive: bool,
        /// Save session answers to a JSON file (default: .conscience/reflections/<session-id>.json)
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
    /// Push a saved snapshot to a dashboard server (run `examine` first)
    Push {
        /// Snapshot id (or unique prefix) or path; defaults to the latest for the project
        snapshot: Option<String>,
        /// Project directory
        #[arg(long)]
        project: Option<PathBuf>,
        /// Dashboard endpoint URL (overrides config)
        #[arg(long)]
        endpoint: Option<String>,
        /// Print exactly what would be sent, as JSON, and do not send it
        #[arg(long)]
        show: bool,
    },
    /// Dump raw ingested data as JSON (debugging; not part of the ordinary workflow)
    #[command(hide = true)]
    Ingest {
        #[command(subcommand)]
        source: IngestSource,
    },

    // ---- Deprecated spellings, kept as hidden aliases for one minor version. ----
    /// Deprecated: use `examine --pr`
    #[command(hide = true)]
    Evaluate {
        #[arg(long)]
        pr: String,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Deprecated: use `examine --all`
    #[command(hide = true, name = "examine-all")]
    ExamineAll {
        #[arg(long, default_value = "30")]
        days: u32,
        #[arg(long)]
        json: bool,
    },
    /// Deprecated: use `examine --all --markdown`
    #[command(hide = true)]
    Digest {
        #[arg(long, default_value = "7")]
        days: u32,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Deprecated: use `report authorship`
    #[command(hide = true)]
    Authorship {
        #[arg(long)]
        repo: String,
        #[arg(long, default_value = "30")]
        days: u32,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Deprecated: use `report attention`
    #[command(hide = true)]
    Attention {
        #[arg(long, default_value = "7")]
        days: u32,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Deprecated: use `report tokens`
    #[command(hide = true, name = "retro-tokens")]
    RetroTokens {
        #[arg(long, default_value = "4")]
        hours: u32,
        #[arg(long)]
        project: Option<PathBuf>,
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
    /// GitHub activity: commits, PRs, reviews
    Github {
        /// GitHub repository (owner/repo). Default: github.repo in conscience.yaml, else the checkout's origin remote
        #[arg(long)]
        repo: Option<String>,
        /// Project directory whose manifest or remote names the repository (default: current directory)
        #[arg(long)]
        project: Option<PathBuf>,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
    },
    /// AI tool usage (all time)
    Ai {
        /// Filter to a specific AI tool (claude-code, copilot, cursor, codex, windsurf, openclaw, nanoclaw)
        #[arg(long, value_parser = parse_ai_tool)]
        tool: Option<String>,
        /// Project directory (default: current directory)
        #[arg(long, conflicts_with = "all")]
        project: Option<PathBuf>,
        /// Every project with data instead of one
        #[arg(long)]
        all: bool,
    },
    /// Estimated energy, CO2, and water for AI usage
    Energy {
        /// Project directory (default: current directory)
        #[arg(long, conflicts_with = "all")]
        project: Option<PathBuf>,
        /// Every project with data instead of one
        #[arg(long)]
        all: bool,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Token retrospective: where did the budget go across recent sessions?
    Tokens {
        /// Window ending now: 90m, 4h, 2d, 1w (bare number = hours)
        #[arg(long, default_value = "4h")]
        since: String,
        /// Project directory (default: current directory)
        #[arg(long, conflicts_with = "all")]
        project: Option<PathBuf>,
        /// Every project with data instead of one
        #[arg(long)]
        all: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Who is writing code vs. operating AI tools
    Authorship {
        /// GitHub repository (owner/repo). Default: github.repo in conscience.yaml, else the checkout's origin remote
        #[arg(long)]
        repo: Option<String>,
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Project directory to match AI sessions (default: all projects)
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Attention and flow: switching between projects (default: all projects)
    Attention {
        /// Number of days to look back
        #[arg(long, default_value = "7")]
        days: u32,
        /// Narrow to one project directory
        #[arg(long)]
        project: Option<PathBuf>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Write an HTML timeline visualization to this path
        #[arg(long)]
        html: Option<PathBuf>,
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

/// One line on stderr for a command spelling that still works but has moved.
fn deprecated(old: &str, new: &str) {
    eprintln!("Note: `conscience {}` is now `conscience {}`; the old form will be removed in a future release.", old, new);
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Setup => run_setup(),
        Commands::Examine {
            repo,
            no_github,
            pr,
            all,
            days,
            project,
            json,
            full,
            markdown,
            output,
            comment,
        } => {
            if let Some(pr) = pr {
                run_evaluate(&pr, project.as_deref(), json, markdown, comment).await
            } else if all {
                run_examine_all(days, json, markdown, output.as_deref()).await
            } else if markdown {
                Err("--markdown needs --all (a digest) or --pr (a comment body)".into())
            } else {
                run_examine(repo.as_deref(), no_github, days, project.as_deref(), json, full).await
            }
        }
        Commands::Report { source } => match source {
            ReportSource::Github {
                repo,
                project,
                days,
            } => run_github_report(repo.as_deref(), project.as_deref(), days).await,
            ReportSource::Ai { tool, project, all } => {
                run_ai_report(tool.as_deref(), project.as_deref(), all)
            }
            ReportSource::Energy {
                project,
                all,
                days,
                json,
            } => run_energy_report(project.as_deref(), all, days, json),
            ReportSource::Tokens {
                since,
                project,
                all,
                json,
            } => run_retro_tokens(&since, project.as_deref(), all, json),
            ReportSource::Authorship {
                repo,
                days,
                project,
                json,
            } => run_authorship(repo.as_deref(), days, project.as_deref(), json).await,
            ReportSource::Attention {
                days,
                project,
                json,
                html,
            } => run_attention(days, project.as_deref(), json, html.as_deref()).await,
        },
        Commands::Reflect {
            repo,
            no_github,
            days,
            project,
            interactive,
            save,
            json,
        } => {
            run_reflect(
                repo.as_deref(),
                no_github,
                days,
                project.as_deref(),
                interactive,
                save,
                json,
            )
            .await
        }
        Commands::Retro { dir, days, json } => run_retro(dir.as_deref(), days, json),
        Commands::Push {
            snapshot,
            project,
            endpoint,
            show,
        } => run_push(snapshot.as_deref(), project.as_deref(), endpoint.as_deref(), show).await,
        Commands::Ingest { source } => match source {
            IngestSource::Github { repo, days } => run_github_ingest(&repo, days).await,
            IngestSource::ClaudeCode { project } => run_claude_code_ingest(project.as_deref()),
        },

        // Deprecated spellings.
        Commands::Evaluate { pr, project, json } => {
            deprecated("evaluate --pr", "examine --pr");
            run_evaluate(&pr, project.as_deref(), json, false, false).await
        }
        Commands::ExamineAll { days, json } => {
            deprecated("examine-all", "examine --all");
            run_examine_all(days, json, false, None).await
        }
        Commands::Digest { days, output, json } => {
            deprecated("digest", "examine --all --markdown");
            run_examine_all(days, json, !json, output.as_deref()).await
        }
        Commands::Authorship {
            repo,
            days,
            project,
            json,
        } => {
            deprecated("authorship", "report authorship");
            run_authorship(Some(&repo), days, project.as_deref(), json).await
        }
        Commands::Attention {
            days,
            project,
            json,
            html,
        } => {
            deprecated("attention", "report attention");
            run_attention(days, project.as_deref(), json, html.as_deref()).await
        }
        Commands::RetroTokens {
            hours,
            project,
            json,
        } => {
            deprecated("retro-tokens", "report tokens --since <hours>h");
            // Old behaviour covered every project; keep that for the alias.
            run_retro_tokens(&format!("{}h", hours), project.as_deref(), project.is_none(), json)
        }
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

    // 2b. Which GitHub repository commands will use here by default
    match project::resolve_project(None).ok().and_then(|s| s.github_repo(None)) {
        Some((repo, source)) => {
            println!("  {} GitHub repository: {} (from {})", ok, repo, source);
        }
        None => {
            println!("  {} GitHub repository (none detected)", fail);
            println!("    \u{2192} Pass --repo, set github.repo in conscience.yaml, or add a GitHub origin remote");
        }
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

/// The repository for a GitHub-only command: the flag, else the manifest,
/// else the current directory's origin remote. Errors if none applies.
fn required_repo(
    explicit: Option<&str>,
    project: Option<&Path>,
) -> Result<String, Box<dyn std::error::Error>> {
    let scope = project::resolve_project(project)?;
    match scope.github_repo(explicit) {
        Some((repo, source)) => {
            if source != project::RepoSource::Flag {
                eprintln!("GitHub repository: {} (from {})", repo, source);
            }
            Ok(repo)
        }
        None => Err(format!(
            "No GitHub repository: pass --repo, set github.repo in conscience.yaml, or run inside a checkout with a GitHub origin remote ({})",
            scope.root.display()
        )
        .into()),
    }
}

async fn run_github_report(
    repo: Option<&str>,
    project: Option<&Path>,
    days: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = required_repo(repo, project)?;
    let interval = Interval::last_days(days);
    let summary = ingest::github::ingest_github(&repo, &interval).await?;
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
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let show_claude = tool.is_none() || tool == Some("claude-code");
    let show_codex = tool.is_none() || tool == Some("codex");
    let scope = report_scope(project, all)?;

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
    all: bool,
    days: u32,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = report_scope(project, all)?;
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
    repo: Option<&str>,
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = required_repo(repo, project)?;
    let interval = Interval::last_days(days);
    let github_summary = ingest::github::ingest_github(&repo, &interval).await?;
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
    snapshot_ref: Option<&str>,
    project: Option<&std::path::Path>,
    endpoint_override: Option<&str>,
    show: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    let scope = project_scope(project)?;

    // Push never re-runs analysis: it uploads a snapshot the user has already
    // been able to inspect, so what lands on the dashboard is exactly what
    // examine showed.
    let (path, snapshot) = match snapshot_ref {
        Some(r) => Snapshot::find(&scope.root, r)?,
        None => Snapshot::latest(&scope.root)?.ok_or_else(|| {
            format!(
                "No snapshots under {}. Run `conscience examine` first, then push.",
                Snapshot::dir_for(&scope.root).display()
            )
        })?,
    };

    let age = chrono::Utc::now() - snapshot.interval.collected_at;
    eprintln!("{}", snapshot.summary());
    eprintln!(
        "  File:     {} (collected {} ago)",
        path.display(),
        humanize_age(age)
    );

    // Only the allowlisted export ever leaves the machine.
    let export = SnapshotExport::from_snapshot(&snapshot);
    eprintln!("  Sanitized: {}", export.sanitization.summary());

    if show {
        println!("{}", serde_json::to_string_pretty(&export)?);
        eprintln!("(--show: nothing was sent)");
        return Ok(());
    }

    let endpoint = endpoint_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("CONSCIENCE_DASHBOARD_URL").ok())
        .or(config.dashboard.endpoint)
        .ok_or("No dashboard endpoint. Set --endpoint, CONSCIENCE_DASHBOARD_URL, or configure in ~/.conscience/config.toml")?;

    let api_key = std::env::var("CONSCIENCE_DASHBOARD_API_KEY")
        .ok()
        .or(config.dashboard.api_key);

    dashboard::push::push_export(&endpoint, &export, api_key.as_deref()).await?;

    Ok(())
}

fn humanize_age(age: chrono::Duration) -> String {
    let mins = age.num_minutes();
    if mins < 60 {
        format!("{} min", mins.max(0))
    } else if mins < 60 * 48 {
        format!("{} h", mins / 60)
    } else {
        format!("{} days", mins / (60 * 24))
    }
}

fn run_retro_tokens(
    since: &str,
    project: Option<&std::path::Path>,
    all: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = report_scope(project, all)?;
    let interval = Interval::since(since)?;
    let summary = ingest::ai::ingest_claude_code(scope.as_ref(), Some(&interval))?;
    let recent = summary.sessions;

    if recent.is_empty() {
        eprintln!("No sessions active in {}.", interval.label());
        std::process::exit(1);
    }

    eprintln!("{} session(s) active in {}", recent.len(), interval.label());

    let retro = analysis::session_retro::analyze_token_retro(&recent);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&retro)?);
        return Ok(());
    }

    println!();
    println!(
        "  Conscience \u{2014} Token Retrospective ({})",
        interval.label()
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


async fn run_examine_all(
    days: u32,
    json_output: bool,
    markdown: bool,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let interval = Interval::last_days(days);
    let analysis = ethics::multi::analyze_all_projects(&interval).await?;

    if analysis.total_projects == 0 {
        eprintln!("No projects found.");
        std::process::exit(1);
    }

    if json_output {
        let json = serde_json::to_string_pretty(&analysis)?;
        match output {
            Some(path) => {
                std::fs::write(path, &json)?;
                eprintln!("Written to {}", path.display());
            }
            None => println!("{}", json),
        }
    } else if markdown {
        let md = ethics::report::render_multi_project_markdown(&analysis, days);
        match output {
            Some(path) => {
                std::fs::write(path, &md)?;
                eprintln!("Digest written to {}", path.display());
            }
            None => print!("{}", md),
        }
    } else {
        ethics::report::print_multi_project_analysis(&analysis);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_reflect(
    repo: Option<&str>,
    no_github: bool,
    days: u32,
    project: Option<&std::path::Path>,
    interactive: bool,
    save: Option<Option<PathBuf>>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = project_scope(project)?;
    let manifest = scope.manifest.clone();
    let manifest_dir = scope.root.clone();

    // Same collection as examine, but reflect does not write a snapshot:
    // it is a conversation aid, not an assessment on the record.
    let collected = pipeline::collect(pipeline::CollectRequest {
        scope: &scope,
        interval: Interval::last_days(days),
        repo,
        pr: None,
        no_github,
    })
    .await?;
    let github_summary = collected.github;
    let ai_summary = collected.ai;

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

            let session =
                ethics::session::ReflectionSession::new(contributor, project_name, responses);

            let save_path = match path_opt {
                Some(p) => p,
                None => {
                    let dir = manifest_dir.join(".conscience").join("reflections");
                    std::fs::create_dir_all(&dir)?;
                    // One file per session, never per day: two sessions on the
                    // same day used to overwrite each other.
                    dir.join(session.default_filename())
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
    markdown: bool,
    comment: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = project_scope(project)?;

    // The pipeline uses the PR's own lifetime as the interval: sessions that
    // overlap the time between it being opened and merged (or closed, or now).
    let collected = pipeline::collect(pipeline::CollectRequest {
        scope: &scope,
        interval: Interval::last_days(30), // replaced by the PR window
        repo: None,
        pr: Some(pr_url),
        no_github: false,
    })
    .await?;

    let Some(github) = collected.github.as_ref() else {
        let why = collected
            .coverage
            .source("github")
            .map(|s| s.detail.clone())
            .unwrap_or_default();
        return Err(format!("Could not fetch the pull request: {}", why).into());
    };

    let pr_number = github.pull_requests[0].number;
    let (owner, repo_name) = (github.owner.clone(), github.repo.clone());

    if !json_output && !markdown {
        let pr = &github.pull_requests[0];
        println!();
        println!("  Conscience \u{2014} PR #{}: \"{}\"", pr.number, pr.title);
        println!(
            "  by {} | +{}/\u{2212}{} | {} files | {} review comments",
            pr.author,
            pr.additions.unwrap_or(0),
            pr.deletions.unwrap_or(0),
            pr.changed_files.unwrap_or(0),
            pr.review_comments,
        );
    }

    let snapshot = pipeline::analyze(&scope, collected);
    let saved = snapshot.save(&scope.root)?;
    eprintln!("{}", snapshot.summary());
    eprintln!("  Saved:    {}", saved.display());

    if json_output {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else if markdown {
        // The comment body, from the sanitized export, printed not posted.
        let export = SnapshotExport::from_snapshot(&snapshot);
        print!("{}", conscience::github::comment::render_pr_comment(&export, pr_number));
    } else {
        ethics::report::print_ethical_analysis(&snapshot.analysis);
    }

    if comment {
        let export = SnapshotExport::from_snapshot(&snapshot);
        eprintln!("  Sanitized: {}", export.sanitization.summary());
        let body = conscience::github::comment::render_pr_comment(&export, pr_number);
        let token = conscience::github::auth::resolve_token()?;
        let client = conscience::github::client::GitHubClient::new(
            conscience::github::auth::build_client(&token)?,
        );
        match client
            .upsert_pr_comment(&owner, &repo_name, pr_number, &body)
            .await?
        {
            conscience::github::comment::CommentOutcome::Created(url) => {
                eprintln!("Posted comment on PR #{}: {}", pr_number, url)
            }
            conscience::github::comment::CommentOutcome::Updated(url) => {
                eprintln!("Updated conscience's comment on PR #{}: {}", pr_number, url)
            }
        }
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
    no_github: bool,
    days: u32,
    project: Option<&std::path::Path>,
    json_output: bool,
    full: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve the project and the interval once; both sources share them.
    let scope = project_scope(project)?;
    if scope.manifest.is_some() {
        eprintln!("Loaded conscience.yaml from {}", scope.root.display());
    }

    let collected = pipeline::collect(pipeline::CollectRequest {
        scope: &scope,
        interval: Interval::last_days(days),
        repo,
        pr: None,
        no_github,
    })
    .await?;

    if collected.github.is_none() && collected.ai.is_none() {
        eprintln!("No data sources available for {}.", scope.root.display());
        eprintln!("  Coverage: {}", collected.coverage.summary());
        eprintln!(
            "  Point --project at a directory with Claude Code sessions, or give a GitHub \
             repository via --repo, github.repo in conscience.yaml, or an origin remote."
        );
        std::process::exit(1);
    }

    // Every examine writes a snapshot: it is the record push uploads and
    // history compares, and it is local (.conscience/ is gitignored).
    let snapshot = pipeline::analyze(&scope, collected);
    let saved = snapshot.save(&scope.root)?;
    eprintln!("{}", snapshot.summary());
    eprintln!("  Saved:    {}", saved.display());

    if json_output {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else if full {
        ethics::report::print_ethical_analysis(&snapshot.analysis);
    } else {
        ethics::report::print_examine_brief(&snapshot);
    }

    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("conscience").chain(args.iter().copied()))
    }

    #[test]
    fn examine_variants_parse() {
        assert!(matches!(
            parse(&["examine"]).unwrap().command,
            Commands::Examine { pr: None, all: false, full: false, .. }
        ));
        assert!(matches!(
            parse(&["examine", "--pr", "org/repo#7", "--json"]).unwrap().command,
            Commands::Examine { pr: Some(_), json: true, .. }
        ));
        assert!(matches!(
            parse(&["examine", "--all", "--markdown", "--output", "d.md", "--days", "7"])
                .unwrap()
                .command,
            Commands::Examine { all: true, markdown: true, output: Some(_), days: 7, .. }
        ));
        assert!(matches!(
            parse(&["examine", "--full", "--repo", "org/repo"]).unwrap().command,
            Commands::Examine { full: true, repo: Some(_), .. }
        ));
    }

    #[test]
    fn examine_rejects_contradictory_flags() {
        assert!(parse(&["examine", "--pr", "x", "--all"]).is_err());
        assert!(parse(&["examine", "--repo", "x", "--pr", "y"]).is_err());
        assert!(parse(&["examine", "--all", "--project", "."]).is_err());
        assert!(parse(&["examine", "--all", "--output", "f"]).is_err(), "output requires --markdown");
        assert!(parse(&["examine", "--full", "--json"]).is_err());
    }

    #[test]
    fn report_subcommands_parse_with_project_or_all() {
        assert!(matches!(
            parse(&["report", "tokens", "--since", "90m"]).unwrap().command,
            Commands::Report { source: ReportSource::Tokens { ref since, all: false, .. } } if since == "90m"
        ));
        assert!(matches!(
            parse(&["report", "tokens"]).unwrap().command,
            Commands::Report { source: ReportSource::Tokens { ref since, .. } } if since == "4h"
        ));
        assert!(matches!(
            parse(&["report", "energy", "--all", "--days", "7"]).unwrap().command,
            Commands::Report { source: ReportSource::Energy { all: true, days: 7, .. } }
        ));
        assert!(parse(&["report", "energy", "--all", "--project", "."]).is_err());
        assert!(matches!(
            parse(&["report", "authorship", "--repo", "o/r"]).unwrap().command,
            Commands::Report { source: ReportSource::Authorship { .. } }
        ));
        assert!(matches!(
            parse(&["report", "attention", "--html", "t.html"]).unwrap().command,
            Commands::Report { source: ReportSource::Attention { html: Some(_), .. } }
        ));
        assert!(matches!(
            parse(&["report", "ai", "--tool", "codex"]).unwrap().command,
            Commands::Report { source: ReportSource::Ai { tool: Some(_), .. } }
        ));
    }

    #[test]
    fn deprecated_spellings_still_parse() {
        assert!(matches!(parse(&["evaluate", "--pr", "o/r#1"]).unwrap().command, Commands::Evaluate { .. }));
        assert!(matches!(parse(&["examine-all", "--days", "3"]).unwrap().command, Commands::ExamineAll { days: 3, .. }));
        assert!(matches!(parse(&["digest"]).unwrap().command, Commands::Digest { days: 7, .. }));
        assert!(matches!(parse(&["authorship", "--repo", "o/r"]).unwrap().command, Commands::Authorship { .. }));
        assert!(matches!(parse(&["attention"]).unwrap().command, Commands::Attention { days: 7, .. }));
        assert!(matches!(parse(&["retro-tokens", "--hours", "2"]).unwrap().command, Commands::RetroTokens { hours: 2, .. }));
        assert!(matches!(parse(&["ingest", "claude-code"]).unwrap().command, Commands::Ingest { .. }));
    }

    #[test]
    fn deprecated_and_ingest_commands_are_hidden_from_help() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let visible: Vec<String> = cmd
            .get_subcommands()
            .filter(|c| !c.is_hide_set())
            .map(|c| c.get_name().to_string())
            .collect();
        assert_eq!(visible, ["setup", "examine", "report", "reflect", "retro", "push"]);
    }

    #[test]
    fn repo_is_optional_and_no_github_is_exclusive_with_it() {
        assert!(matches!(
            parse(&["report", "github"]).unwrap().command,
            Commands::Report { source: ReportSource::Github { repo: None, .. } }
        ));
        assert!(matches!(
            parse(&["report", "authorship", "--days", "7"]).unwrap().command,
            Commands::Report { source: ReportSource::Authorship { repo: None, days: 7, .. } }
        ));
        assert!(matches!(
            parse(&["examine", "--no-github"]).unwrap().command,
            Commands::Examine { no_github: true, repo: None, .. }
        ));
        assert!(parse(&["examine", "--no-github", "--repo", "o/r"]).is_err());
        assert!(parse(&["examine", "--no-github", "--pr", "o/r#1"]).is_err());
        assert!(parse(&["reflect", "--no-github", "--repo", "o/r"]).is_err());
        assert!(matches!(
            parse(&["reflect", "--no-github"]).unwrap().command,
            Commands::Reflect { no_github: true, .. }
        ));
    }

    #[test]
    fn pr_comment_flags_parse_and_require_pr() {
        assert!(matches!(
            parse(&["examine", "--pr", "o/r#4", "--comment"]).unwrap().command,
            Commands::Examine { comment: true, pr: Some(_), .. }
        ));
        assert!(matches!(
            parse(&["examine", "--pr", "o/r#4", "--markdown"]).unwrap().command,
            Commands::Examine { markdown: true, pr: Some(_), .. }
        ));
        assert!(parse(&["examine", "--comment"]).is_err(), "--comment needs --pr");
        assert!(parse(&["examine", "--pr", "o/r#4", "--comment", "--json"]).is_ok());
        assert!(parse(&["examine", "--pr", "o/r#4", "--markdown", "--json"]).is_err());
        // --markdown alone still parses; the dispatcher rejects it with a hint.
        assert!(parse(&["examine", "--markdown"]).is_ok());
    }
}
