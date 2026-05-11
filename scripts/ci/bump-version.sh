#!/usr/bin/env bash
set -euo pipefail

BUMP_TYPE=${1:-patch}

if ! cargo set-version --help >/dev/null 2>&1; then
  echo "❌ cargo set-version not available. Install cargo-edit or upgrade Rust." >&2
  exit 1
fi

# Current version (from [workspace.package])
PREV=$(cargo metadata --no-deps --format-version=1 \
  | jq -r '.workspace_metadata.workspace.package.version // .packages[0].version')

# Bump
cargo set-version --workspace --bump "$BUMP_TYPE" >/dev/null

# New version
NEW=$(cargo metadata --no-deps --format-version=1 \
  | jq -r '.workspace_metadata.workspace.package.version // .packages[0].version')

if [[ "$PREV" == "$NEW" ]]; then
  echo "Version unchanged ($NEW)"
  exit 0
fi

[[ -f Cargo.lock ]] && cargo generate-lockfile >/dev/null

scripts/ci/sync-release-surface-version.sh "$NEW"

cargo test -p canic --test install_script_surface -- --test-threads=1 --nocapture >/dev/null
cargo test -p canic --test protocol_surface -- --test-threads=1 --nocapture >/dev/null

if git rev-parse "v$NEW" >/dev/null 2>&1; then
  echo "❌ Tag v$NEW already exists. Aborting." >&2
  exit 1
fi

echo "✅ Bumped: $PREV → $NEW"
echo "Review changes, then run:"
echo "  make release-stage"
echo "  make release-commit"
echo "  make release-push"
