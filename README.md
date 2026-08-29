# LiteCode CLI (Rust prototype)

A lightweight coding harness with a developer-tool style UI, designed to pair with LiteLLM Auto Router.

## Quick run

```bash
cd litecode-cli
cargo run -- start
cargo run -- preview
cargo run -- run "implement streaming retry fallback" --quality high --budget cheap
```

## V0 shape

- **Preview mode** for branding + architecture output
- **Start mode** for first-run LiteLLM sign-in
- **Plan mode** for a minimal implementation plan
- **Run mode** for a simple harness loop:
  - plan → execute → verify → summarize
- **Router policy** inputs:
  - `--quality fast|balanced|high`
  - `--budget cheapest|cheap|flexible`

## Why this style

- Dark terminal, cyan accents, compact status lines
- Feels like a fast coding devtool (minimal cognitive overhead)
- Keeps the cost value prop obvious via routing and budget controls
