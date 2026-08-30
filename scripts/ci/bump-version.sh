#!/usr/bin/env bash
set -euo pipefail

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

if [[ -n "$(git status --porcelain)" ]]; then
  echo "❌ Refusing to bump a dirty source candidate." >&2
  exit 1
fi

# Current version (from [workspace.package]).
PREV="$(bash "$VERSION_READER")"

IFS=. read -r PREV_MAJOR PREV_MINOR PREV_PATCH <<<"${PREV%%[-+]*}"
case "$BUMP_TYPE" in
  patch)
    PLANNED="$PREV_MAJOR.$PREV_MINOR.$((PREV_PATCH + 1))"
    ;;
  minor)
    PLANNED="$PREV_MAJOR.$((PREV_MINOR + 1)).0"
    ;;
  major)
    PLANNED="$((PREV_MAJOR + 1)).0.0"
    ;;
  *)
    echo "❌ Unsupported version bump: $BUMP_TYPE" >&2
    exit 2
    ;;
esac
PLANNED_MINOR_LINE="${PLANNED%.*}"
DETAILED_CHANGELOG="docs/changelog/$PLANNED_MINOR_LINE.md"
STATUS_DOCUMENT="docs/status/current.md"
STATUS_MARKER_PATTERN='^<!-- canic-release-(state|validation):.*-->$'

bash scripts/ci/check-release-draft-ready.sh "$BUMP_TYPE"

# Refresh remote state after validation and immediately before any version file
# changes. A stale source branch or occupied tag must not leave a local release
# commit/tag that cannot be pushed normally.
bash scripts/ci/check-release-remote-state.sh before-version "$PLANNED"

TRANSACTION_DIR="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-bump.XXXXXX")"
BACKUP_ARCHIVE="$TRANSACTION_DIR/release-surfaces.tar"
mapfile -t RELEASE_SURFACES < <(
  {
    git ls-files -- 'Cargo.toml' ':(glob)**/Cargo.toml'
    printf '%s\n' Cargo.lock scripts/dev/install_dev.sh \
      "$DETAILED_CHANGELOG" "$STATUS_DOCUMENT"
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

[[ -f Cargo.lock ]] && cargo update --workspace --offline >/dev/null

scripts/ci/sync-release-surface-version.sh "$NEW"

release_header="$(rg -m1 "^## ${NEW//./\\.} - (Unreleased|[0-9]{4}-[0-9]{2}-[0-9]{2})$" "$DETAILED_CHANGELOG")"
recorded_release_state="${release_header#"## $NEW - "}"
if [[ "$recorded_release_state" == "Unreleased" ]]; then
  RELEASE_DATE="${CANIC_RELEASE_DATE:-$(date -u +%F)}"
else
  RELEASE_DATE="$recorded_release_state"
fi
[[ "$RELEASE_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || {
  echo "❌ Invalid release date: $RELEASE_DATE" >&2
  rollback_release_surfaces 1
}
sed -i \
  "s/^## $NEW - Unreleased$/## $NEW - $RELEASE_DATE/" \
  "$DETAILED_CHANGELOG"
VALIDATION_STATUS_MARKER="<!-- canic-release-validation: version=$NEW source=$CURRENT_HEAD date=$RELEASE_DATE gate=$VALIDATION_KIND -->"
sed -i -E "/$STATUS_MARKER_PATTERN/d" "$STATUS_DOCUMENT"
printf '\n%s\n' "$VALIDATION_STATUS_MARKER" >>"$STATUS_DOCUMENT"
[[ "$(rg -c -F "$VALIDATION_STATUS_MARKER" "$STATUS_DOCUMENT" || true)" -eq 1 ]] || {
  echo "❌ Failed to bind current status to the validated release candidate." >&2
  rollback_release_surfaces 1
}

[[ "$(rg -c -F "## $NEW - $RELEASE_DATE" "$DETAILED_CHANGELOG")" -eq 1 ]] || {
  echo "❌ Failed to seal $DETAILED_CHANGELOG for $NEW." >&2
  rollback_release_surfaces 1
}
[[ "$(rg -c -F "$VALIDATION_STATUS_MARKER" "$STATUS_DOCUMENT")" -eq 1 ]] || {
  echo "❌ Failed to bind current status to validated source $CURRENT_HEAD." >&2
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
