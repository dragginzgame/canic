#!/usr/bin/env bash
set -euo pipefail

# Point Git to use the repository hooks directory
git config core.hooksPath .githooks

# Ensure hook is executable for this checkout
chmod +x .githooks/pre-commit || true

echo "Git hooks installed. Pre-commit will run 'cargo fmt --all -- --check'."

