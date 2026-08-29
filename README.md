# Shiprr (Rust prototype)

Shiprr is a minimal coding harness with a smart routing layer designed to cut costs on top of LiteLLM Auto Router.

## Install

```bash
cd shiprr
./scripts/install.sh
```

## Start

```bash
shipr
```

`shipr` opens an agentic CLI prompt where plain text is treated as a task.

## CLI flow (Claude/Codex-like)

```text
❯ shipr
❯ fix retry typo in docs
... thought block
... progress block
... recap
❯ /status
❯ /tasks
❯ /exit
```

## Commands

- `/help`
- `/status`
- `/tasks`
- `/preview`
- `/plan <task>`
- `/login`
- `/exit`

## Why Shiprr

- Minimal by design: just an agentic loop
- Harness-level smart routing chooses cheaper viable policy first
- Manual overrides available in direct run mode (`shipr run ... --quality ... --budget ...`)
- Devtool aesthetic: dark, compact, low-noise output

## Architecture

- Binary CLI: `src/main.rs`
- Smart routing crate: `crates/shipr-smart-routing/src/lib.rs`
