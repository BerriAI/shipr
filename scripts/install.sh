#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required. install rustup first: https://rustup.rs"
  exit 1
fi

echo "Installing shipr..."
cargo install --path . --force

echo
echo "Installed. Run:"
echo "  shipr"
echo "or"
echo "  shipr start"
