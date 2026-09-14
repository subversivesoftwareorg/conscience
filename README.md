# Conscience

A CLI tool for evaluating the impact of AI-assisted work — measuring not just velocity and output, but whether AI is serving human flourishing.

Conscience asks the question posed by Pope Leo XIV in *Magnifica Humanitas*: **"Does AI make human life on earth 'more human' in every aspect of that life?"**

## Install

```bash
# macOS / Linux via Homebrew
brew tap subversivesoftwareorg/tap
brew install conscience

# From crates.io
cargo install conscience

# Prebuilt binaries (Linux amd64/arm64, macOS arm64)
# https://github.com/subversivesoftwareorg/conscience/releases
```

## Quick Start

```bash
conscience setup                     # see what's configured
conscience examine --project .       # run your first ethical analysis
conscience reflect -i --project .    # answer reflection questions interactively
```

See the **[Getting Started guide](https://github.com/subversivesoftwareorg/conscience/wiki/Getting-Started)** for a complete walkthrough.

## Documentation

Full documentation lives in the **[Wiki](https://github.com/subversivesoftwareorg/conscience/wiki)**.

### By use case

| I want to... | Guide |
|---|---|
| Set up conscience for the first time | [Getting Started](https://github.com/subversivesoftwareorg/conscience/wiki/Getting-Started) |
| Run a team retrospective | [Running a Team Retrospective](https://github.com/subversivesoftwareorg/conscience/wiki/Running-a-Team-Retrospective) |
| Evaluate a pull request | [Evaluating a Pull Request](https://github.com/subversivesoftwareorg/conscience/wiki/Evaluating-a-Pull-Request) |
| Understand my attention patterns | [Understanding Your Attention Patterns](https://github.com/subversivesoftwareorg/conscience/wiki/Understanding-Your-Attention-Patterns) |
| Measure energy consumption | [Measuring Energy Cost](https://github.com/subversivesoftwareorg/conscience/wiki/Measuring-Energy-Cost) |
| Set up CI/CD analysis | [Setting Up CI/CD](https://github.com/subversivesoftwareorg/conscience/wiki/Setting-Up-CI-CD) |
| Track trends with a dashboard | [Tracking Trends with a Dashboard](https://github.com/subversivesoftwareorg/conscience/wiki/Tracking-Trends-with-a-Dashboard) |

### Reference

| Topic | Page |
|---|---|
| Every command, every flag | [Command Reference](https://github.com/subversivesoftwareorg/conscience/wiki/Command-Reference) |
| conscience.yaml, GitHub auth, thresholds | [Configuration](https://github.com/subversivesoftwareorg/conscience/wiki/Configuration) |
| The ethical framework | [The Seven Principles](https://github.com/subversivesoftwareorg/conscience/wiki/The-Seven-Principles) |
| Security signal detection | [Security Signals](https://github.com/subversivesoftwareorg/conscience/wiki/Security-Signals) |

## Contributing

Conscience is built with Rust. To get started:

```bash
git clone https://github.com/subversivesoftwareorg/conscience
cd conscience
cargo build
cargo test
cargo run -- examine --project .
```

Areas where contributions are especially welcome:

- **AI tool parsers** — Copilot, Cursor, Codex, Windsurf log ingestion (implement the `AiToolParser` trait)
- **Signal detectors** — new patterns worth surfacing
- **Report formats** — Markdown/PDF export for retrospectives
- **Integration** — Claude Code skill, additional CI/CD platforms

## License

MIT
