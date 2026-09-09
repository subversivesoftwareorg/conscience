# Conscience

A tool for evaluating the impact of AI-assisted work — measuring not just velocity and output, but whether AI is serving human flourishing.

## Project Vision

Conscience asks the question posed by Pope Leo XIV in *Magnifica Humanitas*: "Does AI make human life on earth 'more human' in every aspect of that life?" (MH 129). It also heeds the Leiden Declaration's call to "evaluate the ethical consequences of your work, and take action accordingly."

This is not a productivity dashboard. It is a discernment tool. It measures what teams ship *and* interrogates whether that work serves the common good, preserves human agency, and distributes benefit equitably.

### The Name

"Conscience" — from the Latin *conscientia* (shared knowledge, awareness). The project embodies the encyclical's insistence that "we cannot consider AI to be morally neutral" (MH 104) and that every technical tool "embodies choices and priorities through what it measures, ignores and optimizes."

## Ethical Framework

The project is grounded in two reference documents (stored in `refs/`):

1. **Magnifica Humanitas** (Leo XIV, May 2026) — Encyclical on safeguarding the human person in the time of AI. Key principles this project operationalizes:
   - **Human Dignity**: The value of persons does not depend on what they achieve or produce (MH 52). AI impact measurement must not reduce developers to output metrics.
   - **Common Good**: Not the sum of individual benefits, but "a greater good that belongs to everyone" (MH 60). Features shipped must be evaluated for who they serve.
   - **Subsidiarity**: Decisions should be made at the closest level to the persons involved (MH 70). Teams should own their own evaluation, not have it imposed from above.
   - **Solidarity**: "No one is saved alone" (MH 73). Individual productivity gains that come at the cost of team learning or junior developer growth are not gains.
   - **Social Justice**: "Every institution is called to serve the human person" (MH 77). AI tools that concentrate benefit among senior engineers while de-skilling others fail this test.

2. **Leiden Declaration on AI and Mathematics** (June 2026) — Practical recommendations this project enforces:
   - Disclose tool use transparently
   - Retain human responsibility for correctness
   - Affirm the humanity of authorship
   - Put effort into proper attribution
   - Don't believe the hype — verify claims against evidence

## Impact Dimensions

Conscience evaluates AI-assisted work across five axes. No single axis is sufficient alone.

### 1. Velocity & Output
What was shipped? How fast? Measured through:
- Lines of code changed (additions, deletions, modifications)
- PRs opened, reviewed, merged, and time-to-merge
- Features completed per sprint/cycle
- Commit frequency and patterns

**Conscience check**: Raw velocity without direction serves no one. "More power does not necessarily imply something better" (MH 93).

### 2. Quality & Safety
Is the work correct, secure, and maintainable? Measured through:
- Bug rates (pre-release, post-release)
- Security vulnerabilities introduced vs. caught
- Test coverage delta
- Review feedback density and sentiment
- CI/CD pass rates

**Conscience check**: AI-generated code that passes tests but introduces subtle security issues or architectural debt is a net negative.

### 3. Business Value
Does the work advance meaningful goals? Measured through:
- Feature-to-revenue mapping (when available)
- Customer impact metrics
- Strategic goal alignment scores (human-rated)
- Time-to-value for key initiatives

**Conscience check**: Revenue extraction that harms users or externalizes costs fails the common good test.

### 4. Developer Experience
Are humans growing, or being de-skilled? Measured through:
- Developer satisfaction surveys (periodic)
- Learning indicators (new languages, patterns, domains explored)
- Cognitive load proxies (context-switching frequency, session length)
- Junior-to-senior growth trajectories
- Pair programming and collaboration patterns

**Conscience check**: "Current approaches to technology can paradoxically de-skill workers, subject them to automated surveillance and relegate them to rigid and repetitive tasks" (MH 150). If AI is making developers into prompt-jockeys who can't code without it, that's a failure.

### 5. Ethical Implication
Is the work aligned with human flourishing? Evaluated through:
- **Job displacement risk**: Is AI augmenting human capability or replacing meaningful work? Are humans still learning?
- **Code provenance**: IP/licensing concerns, whether AI-generated code introduces unknown patterns or dependencies
- **Equity of benefit**: Are AI tools benefiting all team members, or concentrating advantage? Are early-career developers being supported or sidelined?
- **Environmental cost**: Token usage, compute costs, energy implications of AI-assisted workflows
- **Transparency**: Is AI involvement disclosed? Can stakeholders see what was human-directed vs. machine-generated?

**Conscience check**: "The digital economy's functioning relies on the silent work of millions of people engaged in essential yet largely unseen activities" (MH 173). Who bears the hidden costs of our AI usage?

## Architecture

### Tech Stack
Rust CLI. Key dependencies:
- **clap** — CLI argument parsing with derive macros
- **octocrab** — Async GitHub API client
- **tokio** — Async runtime
- **serde/serde_json** — Serialization
- **chrono** — Date/time handling
- **comfy-table** — Terminal table rendering
- **colored** — Terminal colors

Build: `cargo build`
Run: `cargo run -- <command>`
Install: `cargo install --path .`

### Data Sources

#### GitHub (repos, PRs, commits, CI)
- Git history analysis: commit frequency, diff sizes, contributor patterns
- PR metadata: review cycles, comment density, approval patterns
- CI/CD logs: build times, test results, deployment frequency
- GitHub API for programmatic access

#### Claude Code Logs
- Session transcripts in `~/.claude/projects/` contain full conversation history
- Extractable metrics: tokens consumed, tools called, code generated vs. accepted
- Session duration, human-vs-AI turn ratios
- Privacy-sensitive — must be handled with explicit consent and configurable anonymization

#### Other AI Tool Signals
- Copilot: telemetry where available, commit message patterns ("Generated by Copilot")
- Cursor/Windsurf: session logs if accessible
- Generic heuristics: detecting AI-generated code patterns

#### Project Management
- Jira/Linear integration for tying commits to tickets to business goals
- Sprint velocity correlation with AI adoption patterns

### Interfaces

1. **CLI Tool** (primary): Commands for ingesting data, running analysis, generating reports
2. **Claude Code Skill**: Invoke analysis during development sessions via `/conscience`
3. **Periodic Reports**: Markdown/PDF summaries for team retrospectives, leadership reviews

## Development Principles

### For This Codebase
- **Practice what we preach**: This project is itself AI-assisted. We disclose that openly and track our own metrics.
- **Privacy by design**: Never store raw conversation transcripts. Extract metrics, discard content. All data collection requires explicit opt-in.
- **No surveillance tool**: Conscience is for teams to evaluate themselves, not for management to monitor individuals. Reports should aggregate, not surveil.
- **Subsidiarity in design**: Teams configure their own evaluation criteria. No one-size-fits-all scoring.
- **Honest measurement**: "It is important to move beyond the current metrics of development" (MH 159). Don't optimize for easily-measured proxy metrics.

### Guiding Questions (from the references)
When evaluating a feature, PR, or sprint, Conscience should help teams ask:
1. Did this work serve the people who will use it, or just the metrics that track it?
2. Are team members growing through this work, or becoming dependent on tools they don't understand?
3. Who benefits from this? Who bears the costs?
4. Would we be comfortable disclosing exactly how AI was used here?
5. Is this building Jerusalem (shared responsibility, piece by piece) or Babel (impressive but dehumanizing)?

## Project Status

**Phase**: Platform — Abuse detection (tokenmaxxing, secret exposure, suspicious bash, prompt injection), multi-project analysis, GitHub Actions integration, dashboard push command.

## Commands

```
# GitHub
conscience ingest github --repo <owner/repo> --days 30
conscience report github --repo <owner/repo> --days 30

# AI Tool Usage (Claude Code implemented; Copilot, Cursor, Codex, Windsurf planned)
conscience ingest claude-code [--project <path>]
conscience report ai [--tool claude-code] [--project <path>]

# Ethical Analysis (signals + scorecard + reflection questions)
conscience examine [--repo <owner/repo>] [--project <path>] [--days 30] [--json]

# Reflection questions for team retrospectives (works with zero data; data enriches)
# --interactive answers each question at a prompt and prints a session summary
conscience reflect [--repo <owner/repo>] [--project <path>] [--days 30] [--interactive] [--json]

# Cross-project analysis across all Claude Code projects
conscience examine-all [--days 30] [--json]

# Push results to a dashboard server
conscience push --repo <owner/repo> --project <path> [--endpoint <url>]

# Attention & flow analysis across projects
conscience attention [--days 7] [--project <path>] [--json] [--html <output.html>]
```

### Planned Commands
```
conscience evaluate --pr <url>
```

## References

- `refs/Encyclical Letter of His Holiness Leo XIV Magnifica Humanitas (15 May 2026).pdf`
- `refs/Leiden_Declaration_on_Artificial_Intelligence_and_Mathematics.pdf`
