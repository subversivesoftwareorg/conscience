# Conscience

A tool for evaluating the impact of AI-assisted work — measuring not just velocity and output, but whether AI is serving human flourishing.

Conscience asks the question posed by Pope Leo XIV in *Magnifica Humanitas*: **"Does AI make human life on earth 'more human' in every aspect of that life?"**

## What It Does

Conscience pulls data from GitHub and AI tool logs, then runs a three-layer ethical analysis:

1. **Signals** — Automated pattern detection: contribution concentration, review gaps, AI dependency ratios, token consumption. Flags concerns *and* healthy patterns.
2. **Scorecard** — Maps signals to six ethical principles drawn from [Magnifica Humanitas](https://www.vatican.va/content/leo-xiv/en/encyclicals/documents/20260515-magnifica-humanitas.html) and the [Leiden Declaration on AI and Mathematics](https://leidendeclaration.ai). Marks which dimensions need human assessment.
3. **Reflection Questions** — Data-informed questions for team retrospectives. Populated with real numbers but answered by humans, not algorithms.

This is a discernment tool, not a surveillance tool. It is designed for teams to evaluate themselves.

## Install

```bash
# From GitHub (recommended for now)
cargo install --git https://github.com/subversivesoftwareorg/conscience

# From crates.io (once published)
cargo install conscience

# From source
git clone https://github.com/subversivesoftwareorg/conscience
cd conscience
cargo install --path .
```

Prebuilt binaries for Linux and macOS are available on the [Releases page](https://github.com/subversivesoftwareorg/conscience/releases).

## Quick Start

### Ethical Analysis

The core command. Combines GitHub data and AI tool logs into a single analysis:

```
conscience examine --repo your-org/your-repo --project /path/to/project
```

This produces signals, a scorecard, and reflection questions. You can use either data source alone:

```
# GitHub data only
conscience examine --repo your-org/your-repo --days 14

# AI tool logs only
conscience examine --project /path/to/project

# JSON output (for piping to other tools)
conscience examine --repo your-org/your-repo --json
```

### Team Reflection

Generate just the reflection questions, formatted for a retrospective. Works with
no data at all — the questions stand on their own — but GitHub activity, AI session
logs, and a `conscience.yaml` manifest enrich them with real context:

```
# Questions enriched with GitHub and AI session data
conscience reflect --repo your-org/your-repo --project /path/to/project

# Bare questions, no data required
conscience reflect

# JSON output
conscience reflect --project /path/to/project --json
```

Add `--interactive` (`-i`) to answer each question at a prompt and get a
session summary. Answers are multi-line: a blank line finishes an answer,
and pressing Enter right away skips the question. With `--json`, the
answered session is emitted as JSON instead — handy for saving:

```
conscience reflect -i --project /path/to/project
conscience reflect -i --json > retro-$(date +%F).json
```

### Attention & Flow

Analyze how your time and attention move across projects:

```
# Weekly attention summary (default 7 days)
conscience attention

# With an HTML timeline visualization
conscience attention --days 14 --html attention.html

# JSON for scripting
conscience attention --json
```

### GitHub Reports

```
# Formatted summary: commits, PRs, contributors
conscience report github --repo your-org/your-repo --days 30

# Raw JSON (pipe to jq, save to file, etc.)
conscience ingest github --repo your-org/your-repo --days 30
```

### AI Tool Usage Reports

```
# Claude Code usage across all projects
conscience report ai

# Filtered to a specific project
conscience report ai --project /path/to/project

# Filtered to a specific tool
conscience report ai --tool claude-code

# Raw JSON
conscience ingest claude-code --project /path/to/project
```

## GitHub Authentication

Conscience tries three methods in order:

1. **`gh` CLI** — If you have [GitHub CLI](https://cli.github.com/) installed and authenticated, it just works. Zero config.
2. **Environment variable** — Set `CONSCIENCE_GITHUB_TOKEN` to a personal access token.
3. **Config file** — Create `~/.conscience/config.toml`:
   ```toml
   [github]
   token = "ghp_your_token_here"
   ```

## AI Tool Support

| Tool | Status | Data Source |
|------|--------|------------|
| Claude Code | Implemented | `~/.claude/projects/` session logs |
| GitHub Copilot | Planned | — |
| Cursor | Planned | — |
| OpenAI Codex | Planned | — |
| Windsurf | Planned | — |

Adding a new AI tool parser means implementing the `AiToolParser` trait — roughly 100-200 lines of Rust.

### Cross-Project Analysis

Scan all your Claude Code projects at once, identify outliers:

```
# See all projects ranked by token usage, with outlier detection
conscience examine-all

# JSON for piping to dashboards
conscience examine-all --json
```

### Dashboard Integration

Push analysis results to a central dashboard server for historical tracking:

```
# Configure endpoint
export CONSCIENCE_DASHBOARD_URL=https://your-dashboard.example.com

# Push analysis
conscience push --repo your-org/your-repo --project /path/to/project
```

Or configure in `~/.conscience/config.toml`:
```toml
[dashboard]
endpoint = "https://your-dashboard.example.com"
api_key = "your-api-key"
```

### Authorship Analysis

Detect whether developers are writing code or operating AI tools:

```
conscience authorship --repo your-org/your-repo --project /path/to/project --days 30
```

This cross-references GitHub commit timestamps with Claude Code session timestamps to calculate an AI authorship correlation percentage per contributor.

### GitHub Actions

Copy `.github/workflows/conscience.yml` into your repo. It runs ethical analysis on PRs and weekly, posting results as GitHub annotations and PR comments.

## The Seven Principles

Each principle maps to a concrete source in the reference documents:

| Principle | Question | Source |
|-----------|----------|--------|
| **Human Agency** | Are humans directing the work, or becoming dependent? | MH 150 |
| **Equity of Benefit** | Are AI tools benefiting all team members? | MH 73, 77 |
| **Transparency** | Is AI involvement disclosed? | Leiden O1 |
| **Developer Growth** | Are team members learning, or being de-skilled? | MH 52, 129 |
| **Environmental Cost** | Is AI usage proportionate to value delivered? | MH 101 |
| **Code Provenance** | Do humans understand and own AI-generated code? | Leiden O4-O6 |
| **Security** | Are AI tools being used safely? Signs of misuse or data exposure? | MH 104, Leiden O4 |

### Security Signal Detection

Conscience detects abuse patterns automatically:

- **Tokenmaxxing** — Sessions with high token-to-file ratios (lots of compute, nothing to show), excessive tokens per turn, or 12+ hour unattended sessions
- **Sensitive file access** — AI reading or writing `.env`, credentials, keys, secrets
- **Suspicious bash** — Network exfiltration patterns (piped curl, netcat), encoding/obfuscation (base64 piping), credential directory access
- **Prompt injection** — PR descriptions containing instruction override patterns, zero-width characters, or conversation injection attempts

## Example Output

```
  Conscience — Ethical Analysis

  Signals

  WARNING High contribution concentration [Equity of Benefit]
          Top 2 of 8 contributors account for 87% of commits.
          AI may be amplifying existing imbalances.
          Evidence: 142 commits across 8 authors

  CONCERN Low review engagement [Human Agency]
          78% of merged PRs had zero review comments.
          Are humans reviewing AI-generated code, or rubber-stamping it?
          Evidence: 14 of 18 merged PRs with no review comments

  HEALTHY Balanced AI:Human interaction [Human Agency]
          AI:Human ratio of 1.4:1 suggests humans are directing the work.
          Evidence: 136 assistant turns vs 99 human turns

  HEALTHY Good cache efficiency [Environmental Cost]
          95% cache hit rate — reusing context rather than recomputing it.
          Evidence: 14.5M cache reads vs 0.8M cache creates

  Scorecard

  ╭────────────────────┬─────────┬───────────────────╮
  │ Principle          │ Signals │ Status            │
  ├────────────────────┼─────────┼───────────────────┤
  │ Human Agency       │ 2       │ CONCERN           │
  │ Equity of Benefit  │ 1       │ WARNING           │
  │ Transparency       │ 1       │ INFO              │
  │ Developer Growth   │ 0       │ Needs human input │
  │ Environmental Cost │ 1       │ HEALTHY           │
  │ Code Provenance    │ 0       │ Needs human input │
  ╰────────────────────┴─────────┴───────────────────╯

  Reflection Questions
  For team discussion — not automated judgment.

  1. Are team members learning new skills through this work,
     or becoming more dependent on AI?

  2. Who benefits from the work shipped this period?

  3. Would we be comfortable if a stakeholder asked exactly
     how AI was used in this work?

  4. Is the AI compute consumed proportionate to the value
     delivered?

  5. Is this work building Jerusalem — shared responsibility,
     piece by piece? Or is it building Babel — impressive but
     concentrated, optimizing for output over human connection?
```

## Ethical Framework

Conscience is grounded in two documents (included in `refs/`):

**Magnifica Humanitas** (Pope Leo XIV, May 2026) — An encyclical on safeguarding the human person in the time of artificial intelligence. It provides five principles that Conscience operationalizes: human dignity, common good, subsidiarity, solidarity, and social justice.

**Leiden Declaration on AI and Mathematics** (June 2026) — Practical recommendations from the mathematical community on responsible AI use: disclose tool use, retain responsibility for correctness, affirm the humanity of authorship, and evaluate ethical consequences.

The project name comes from the encyclical's insistence that "crucial questions impose themselves on our conscience and can no longer be avoided: Where are we going? Toward what goal do we wish to orient ourselves?" (MH 6).

## Contributing

Conscience is built with Rust. To get started:

```
git clone <repo-url>
cd conscience
cargo build
cargo run -- examine --project .
```

Areas where contributions are especially welcome:

- **AI tool parsers** — Copilot, Cursor, Codex, Windsurf log ingestion
- **Signal detectors** — New patterns worth surfacing
- **Report formats** — Markdown export, PDF generation for retrospectives
- **Integration** — Claude Code skill, CI/CD integration

## License

MIT
