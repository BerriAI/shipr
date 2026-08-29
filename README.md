# Shiprr (Rust prototype)

Shiprr is a minimal coding harness with a smart routing layer designed to cut costs on top of LiteLLM Auto Router.

The visual identity uses a blue cargo ship and container motif. The workspace is
dark, sparse, and terminal-native, inspired by the restraint of tools like Ghostty.

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
│                  ╭─────╮                             │
│      ╭─────╭─────┤ ▦ ▦ │╭─────╮                     │
│      ╰─────┴─────┴─────┴┴─────╯                     │
│  ≋≋≋   ╲___  S H I P R R  _______/►                 │
│ ≋≋≋≋≋≋≋≋╲________________/≋≋≋≋≋≋≋≋                 │
│ › fix retry typo in docs                              │
│                                                      │
│ ✦ Processing… Reading context                        │
│                                                      │
│ ● Response streams here token by token               │
├ Ask Shiprr ──────────────────────────────────────────┤
│ › type your next task                                │
└──────────────────────────────────────────────────────┘
```

While a task runs, one animated `Processing…` row changes from routing to
reading, planning, and responding. These transient stages are not retained in
the transcript. The answer streams into the conversation as it is generated.
Real tool calls will be retained once tool execution is wired. Press `Esc` to
cancel or `Ctrl+C` to exit.

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
