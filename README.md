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

`shipr` opens a full-screen agentic terminal UI. The conversation and work log
scroll above a persistent composer at the bottom of the screen.

## CLI flow (Claude/Codex-like)

```text
┌ shiprr ───────────────────────────────────────────────┐
│ › fix retry typo in docs                              │
│                                                      │
│ Thought for a moment                                 │
│ ● Route   Classified task and selected a model       │
│ ● Inspect Read repository context                    │
│ ● Work    Prepared edits and verification            │
│                                                      │
│ ● Response appears here                              │
├ Ask Shiprr ──────────────────────────────────────────┤
│ › type your next task                                │
└──────────────────────────────────────────────────────┘
```

While a task runs, an animated `Processing…` row appears directly above the
composer. Press `Esc` to cancel or `Ctrl+C` to exit.

## Commands

- `/help`
- `/clear`
- `/exit`

## Why Shiprr

- Minimal by design: just an agentic loop
- Harness-level smart routing chooses cheaper viable policy first
- Manual overrides available in direct run mode (`shipr run ... --quality ... --budget ...`)
- IDE-like terminal surface with a fixed composer and streaming work feed

## Architecture

- Binary CLI: `src/main.rs`
- Smart routing crate: `crates/shipr-smart-routing/src/lib.rs`
