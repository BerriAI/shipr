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
scroll above a persistent composer at the bottom of the screen. Shiprr operates
on the directory where it was started.

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

While a task runs, one animated `Processing…` row shows the current model or
tool action. Transient activity is not retained in the transcript. Answers
stream into the conversation as they are generated, while completed tool calls
remain in the feed. Press `Esc` to cancel or `Ctrl+C` to exit.

## Coding tools

Shiprr sends OpenAI-compatible tool definitions to LiteLLM and repeats the
model → tool → result loop until the task is complete. The minimal tool set is:

- `list_files` — discover the workspace
- `read_file` — inspect text with line numbers
- `search` — search with ripgrep
- `write_file` — create or overwrite a file
- `replace_in_file` — make one exact targeted edit
- `run_command` — execute one program directly in the workspace

Reads run immediately. File changes and commands pause on a visible `y/n`
approval prompt. Paths are restricted to the current workspace and `.git`
cannot be edited by file tools.

Responses and tool calls stream from LiteLLM using the OpenAI-compatible
`/v1/chat/completions` endpoint.

## Commands

- `/help`
- `/clear`
- `/exit`

Run `shipr login` to replace saved LiteLLM credentials or change the model.
Existing configs without a model use `auto_router1`. `SHIPR_MODEL` overrides
that default. Optional `SHIPR_FAST_MODEL`, `SHIPR_BALANCED_MODEL`, and
`SHIPR_HIGH_MODEL` environment variables map harness routing tiers to separate
gateway model aliases.

## Why Shiprr

- Minimal by design: just an agentic loop
- Harness-level smart routing chooses cheaper viable policy first
- Manual overrides available in direct run mode (`shipr run ... --quality ... --budget ...`)
- IDE-like terminal surface with a fixed composer and streaming work feed
- Workspace-scoped tools with explicit approval for writes and commands

## Architecture

- Binary CLI: `src/main.rs`
- Agent loop: `src/agent.rs`
- Coding tools: `src/tools.rs`
- Smart routing crate: `crates/shipr-smart-routing/src/lib.rs`
