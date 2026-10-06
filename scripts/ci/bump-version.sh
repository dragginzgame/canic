#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/ci/require-jq.sh
source "$(dirname "${BASH_SOURCE[0]}")/../ci/require-jq.sh"

BUMP_TYPE=${1:-patch}

if [[ "${CANIC_RELEASE_VALIDATED:-}" != "1" ]]; then
  echo "❌ Refusing to bump before a governed release validation lane passes." >&2
  echo "Use make patch, make patch-fast, make minor, or make major." >&2
  exit 1
fi

if [[ -z "${CANIC_RELEASE_VALIDATED_HEAD:-}" ]]; then
  echo "❌ Refusing to bump without the exact validated source revision." >&2
  echo "Use make patch, make minor, or make major." >&2
  exit 1
fi

if ! cargo set-version --help >/dev/null 2>&1; then
  echo "❌ cargo set-version not available. Install cargo-edit or upgrade Rust." >&2
  exit 1
fi

require_jq
ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"
VERSION_READER="$ROOT_DIR/scripts/ci/read-workspace-version.sh"
CURRENT_HEAD="$(git rev-parse HEAD)"
VALIDATION_KIND="${CANIC_RELEASE_VALIDATION_KIND:-complete}"

case "$VALIDATION_KIND" in
  complete | fast) ;;
  *)
    echo "❌ Unsupported release validation kind: $VALIDATION_KIND" >&2
    exit 1
    ;;
esac

if [[ "$BUMP_TYPE" != "patch" && "$VALIDATION_KIND" != "complete" ]]; then
  echo "❌ Fast validation is available only for patch releases." >&2
  exit 1
fi

if [[ "$CANIC_RELEASE_VALIDATED_HEAD" != "$CURRENT_HEAD" ]]; then
  echo "❌ Validated source revision is stale or mismatched." >&2
  exit 1
fi

if [[ -z "${RELEASE_VERSION:-}" && -n "$(git status --porcelain)" ]]; then
  echo "❌ Refusing to bump a dirty source candidate." >&2
  exit 1
fi

# Current version (from [workspace.package]).
PREV="$(bash "$VERSION_READER")"

PLANNED="$(bash scripts/ci/next-release-version.sh "$PREV" "$BUMP_TYPE")"
if [[ -n "${RELEASE_VERSION:-}" && "$RELEASE_VERSION" != "$PLANNED" ]]; then
  echo "❌ Release identity mismatch: requested $RELEASE_VERSION, but $PREV with $BUMP_TYPE plans $PLANNED." >&2
  exit 1
fi
PLANNED_MINOR_LINE="${PLANNED%.*}"
DETAILED_CHANGELOG="docs/changelog/$PLANNED_MINOR_LINE.md"
VALIDATION_RECEIPT="release-validation.json"

bash scripts/ci/check-release-draft-ready.sh "$BUMP_TYPE"

# Refresh remote state after validation and immediately before any version file
# changes. A stale source branch or occupied tag must not leave a local release
# commit/tag that cannot be pushed normally.
if [[ -z "${RELEASE_VERSION:-}" ]]; then
  bash scripts/ci/check-release-remote-state.sh before-version "$PLANNED"
fi

TRANSACTION_DIR="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-bump.XXXXXX")"
BACKUP_ARCHIVE="$TRANSACTION_DIR/release-surfaces.tar"
NOTES_EXISTED=0
[[ ! -f "$DETAILED_CHANGELOG" ]] || NOTES_EXISTED=1
RECEIPT_EXISTED=0
[[ ! -f "$VALIDATION_RECEIPT" ]] || RECEIPT_EXISTED=1
RELEASE_SURFACES=()
while IFS= read -r path; do RELEASE_SURFACES[${#RELEASE_SURFACES[@]}]="$path"; done < <(
  {
    git ls-files -- 'Cargo.toml' ':(glob)**/Cargo.toml'
    printf '%s\n' Cargo.lock CHANGELOG.md scripts/dev/install_dev.sh
    if [[ "$NOTES_EXISTED" -eq 1 ]]; then printf '%s\n' "$DETAILED_CHANGELOG"; fi
    if [[ "$RECEIPT_EXISTED" -eq 1 ]]; then printf '%s\n' "$VALIDATION_RECEIPT"; fi
  } | sort -u
)
tar -cf "$BACKUP_ARCHIVE" "${RELEASE_SURFACES[@]}"
cargo metadata --locked --offline --no-deps --format-version 1 > "$TRANSACTION_DIR/metadata.json"
cp -p Cargo.lock "$TRANSACTION_DIR/Cargo.lock"

rollback_release_surfaces() {
  local status="${1:-1}"

  trap - ERR INT TERM
  if [[ "$NOTES_EXISTED" -eq 0 ]]; then rm -f -- "$DETAILED_CHANGELOG"; fi
  if [[ "$RECEIPT_EXISTED" -eq 0 ]]; then rm -f -- "$VALIDATION_RECEIPT"; fi
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
cargo set-version --workspace --offline "$PLANNED" >/dev/null

# New version.
NEW="$(bash "$VERSION_READER")"

if [[ "$PREV" == "$NEW" ]]; then
  finish_release_surface_transaction
  echo "Version unchanged ($NEW)"
  exit 0
fi

if [[ "$NEW" != "$PLANNED" ]]; then
  echo "❌ Cargo produced $NEW but the governed release draft is $PLANNED." >&2
  rollback_release_surfaces 1
fi

perl scripts/release/retain-lock-selection.pl "$TRANSACTION_DIR/metadata.json" "$TRANSACTION_DIR/Cargo.lock" "$PREV" "$NEW" > Cargo.lock
cargo metadata --locked --offline --no-deps --format-version 1 >/dev/null

scripts/ci/sync-release-surface-version.sh "$NEW"

RELEASE_DATE="${CANIC_RELEASE_DATE:-$(date -u +%F)}"
[[ "$RELEASE_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || rollback_release_surfaces 1
mkdir -p "$(dirname "$DETAILED_CHANGELOG")"
if [[ "$NOTES_EXISTED" -eq 0 ]]; then printf '# %s\n\n' "$PLANNED_MINOR_LINE" > "$DETAILED_CHANGELOG"; fi
for notes in CHANGELOG.md "$DETAILED_CHANGELOG"; do
  awk -v version="$NEW" -v date="$RELEASE_DATE" \
    -f scripts/ci/finalize-release-changelog.awk "$notes" > "$TRANSACTION_DIR/notes"
  cat "$TRANSACTION_DIR/notes" > "$notes"
done
# Keep validation provenance separate from the editable developer handoff.
# jq owns these literal variable names.
# shellcheck disable=SC2016
"$JQ_BIN" -n --arg version "$NEW" --arg source "$CURRENT_HEAD" \
  --arg date "$RELEASE_DATE" --arg gate "$VALIDATION_KIND" \
  '{schema: 1, version: $version, source: $source, date: $date, gate: $gate}' \
  >"$TRANSACTION_DIR/validation.json"
mv "$TRANSACTION_DIR/validation.json" "$VALIDATION_RECEIPT"

[[ "$(rg -c -F "## [$NEW] - $RELEASE_DATE" "$DETAILED_CHANGELOG")" -eq 1 ]] || {
  echo "❌ Failed to seal $DETAILED_CHANGELOG for $NEW." >&2
  rollback_release_surfaces 1
}
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
