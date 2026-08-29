# Routr (Rust prototype)

Routr is a minimal coding harness with a smart routing layer designed to cut costs on top of LiteLLM Auto Router.

## Install

```bash
cd litecode-cli
./scripts/install.sh
```

## Start

```bash
routr
```

`routr` with no args starts first-run sign-in.

## Core commands

```bash
routr start
routr preview
routr run "fix readme typo"
routr run "investigate race condition in retries"
routr run "implement streaming retry fallback" --quality high --budget cheap
```

## Why Routr

- Minimal by design: just an agentic loop
- Harness-level smart routing chooses cheaper viable policy first
- Manual overrides stay available with `--quality` and `--budget`
- Devtool aesthetic: dark, compact, low-noise output

## Architecture

- Binary CLI: `src/main.rs`
- Smart routing crate: `crates/routr-smart-routing/src/lib.rs`
