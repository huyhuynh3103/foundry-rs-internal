#!/bin/bash
set -euo pipefail

cd /Users/huy.huynh/repo/rust/fdk

# Create fdk branch if not already on it
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "none")
if [ "$CURRENT_BRANCH" != "fdk" ]; then
  git checkout -b fdk 2>/dev/null || git checkout fdk
fi

# Ensure the project compiles (incremental — fast after first build)
echo "Checking cheatcodes crate compiles..."
cargo check -p foundry-cheatcodes -p foundry-cheatcodes-spec 2>&1 || {
  echo "First build — this may take a few minutes..."
  cargo build -p foundry-cheatcodes
}

echo "FDK environment ready."
