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

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

# Current version (from [workspace.package]).
PREV=$(cargo get workspace.package.version)

TRANSACTION_DIR="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-bump.XXXXXX")"
BACKUP_ARCHIVE="$TRANSACTION_DIR/release-surfaces.tar"
mapfile -t RELEASE_SURFACES < <(
  {
    git ls-files -- 'Cargo.toml' ':(glob)**/Cargo.toml'
    printf '%s\n' Cargo.lock scripts/dev/install_dev.sh
  } | sort -u
)
tar -cf "$BACKUP_ARCHIVE" "${RELEASE_SURFACES[@]}"

rollback_release_surfaces() {
  local status="${1:-1}"

  trap - ERR INT TERM
  tar -xf "$BACKUP_ARCHIVE" -C "$ROOT_DIR"
  rm -rf "$TRANSACTION_DIR"
  echo "❌ Version bump failed; restored all release surfaces to $PREV." >&2
  exit "$status"
}

finish_release_surface_transaction() {
  trap - ERR INT TERM
  rm -rf "$TRANSACTION_DIR"
}

trap 'rollback_release_surfaces $?' ERR
trap 'rollback_release_surfaces 130' INT
trap 'rollback_release_surfaces 143' TERM

# Bump
cargo set-version --workspace --bump "$BUMP_TYPE" >/dev/null

# New version.
NEW=$(cargo get workspace.package.version)

if [[ "$PREV" == "$NEW" ]]; then
  finish_release_surface_transaction
  echo "Version unchanged ($NEW)"
  exit 0
fi

[[ -f Cargo.lock ]] && cargo generate-lockfile >/dev/null

scripts/ci/sync-release-surface-version.sh "$NEW"

if git rev-parse "v$NEW" >/dev/null 2>&1; then
  echo "❌ Tag v$NEW already exists. Aborting." >&2
  rollback_release_surfaces 1
fi

finish_release_surface_transaction

echo "✅ Bumped: $PREV → $NEW"
echo "Next:"
echo "  git diff"
echo "  make release-stage"
echo "  make release-commit"
echo "  make release-push"
