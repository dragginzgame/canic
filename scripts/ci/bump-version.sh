#!/usr/bin/env bash
set -euo pipefail

BUMP_TYPE=${1:-patch}

if [[ "${CANIC_RELEASE_GATES_PASSED:-}" != "1" ]]; then
  echo "❌ Refusing to bump before release gates pass." >&2
  echo "Use make patch, make minor, or make major." >&2
  exit 1
fi

if ! cargo set-version --help >/dev/null 2>&1; then
  echo "❌ cargo set-version not available. Install cargo-edit or upgrade Rust." >&2
  exit 1
fi

# Current version (from [workspace.package]).
PREV=$(cargo get workspace.package.version)

# Bump
cargo set-version --workspace --bump "$BUMP_TYPE" >/dev/null

# New version.
NEW=$(cargo get workspace.package.version)

if [[ "$PREV" == "$NEW" ]]; then
  echo "Version unchanged ($NEW)"
  exit 0
fi

[[ -f Cargo.lock ]] && cargo generate-lockfile >/dev/null

scripts/ci/sync-release-surface-version.sh "$NEW"

if git rev-parse "v$NEW" >/dev/null 2>&1; then
  echo "❌ Tag v$NEW already exists. Aborting." >&2
  exit 1
fi

echo "✅ Bumped: $PREV → $NEW"
echo "Next:"
echo "  git diff"
echo "  make release-stage"
echo "  make release-commit"
echo "  make release-push"
