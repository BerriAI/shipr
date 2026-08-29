# Shipr (Rust prototype)

Shipr is a minimal coding harness with a smart routing layer designed to cut costs on top of LiteLLM Auto Router.

## Install

```bash
cd shippr
./scripts/install.sh
```

## Start

```bash
shipr
```

`shipr` with no args starts an agentic `shipr>` shell.
On first run, it prompts sign-in before entering the shell.

## Interactive shell

Inside the shell:

```text
shipr> fix readme typo
shipr> investigate race condition in retries
shipr> /status
shipr> /tasks
shipr> /plan refactor retry flow
shipr> /exit
```

Any plain text input is treated as a task and runs through the agentic loop with progress updates.

## Core commands

```bash
shipr start
shipr preview
shipr run "fix readme typo"
shipr run "investigate race condition in retries"
shipr run "implement streaming retry fallback" --quality high --budget cheap
```

## Why Shipr

- Minimal by design: just an agentic loop
- Harness-level smart routing chooses cheaper viable policy first
- Manual overrides stay available with `--quality` and `--budget`
- Devtool aesthetic: dark, compact, low-noise output

## Architecture

- Binary CLI: `src/main.rs`
- Smart routing crate: `crates/shipr-smart-routing/src/lib.rs`
