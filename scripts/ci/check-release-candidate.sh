#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALL_DEV="$ROOT/scripts/dev/install_dev.sh"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"
STATUS_DOCUMENT="$ROOT/docs/status/current.md"

fail() {
    echo "release candidate guard failed: $1" >&2
    exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo is unavailable"
command -v jq >/dev/null 2>&1 || fail "jq is unavailable"
command -v rg >/dev/null 2>&1 || fail "rg is unavailable"

workspace_version="$(bash "$VERSION_READER")" ||
    fail "cargo-get could not read the root workspace version"
minor_line="${workspace_version%.*}"
detailed_changelog="$ROOT/docs/changelog/$minor_line.md"

[ -f "$detailed_changelog" ] ||
    fail "detailed changelog is missing for $workspace_version"
release_header="$(rg -m1 -F "## $workspace_version - " "$detailed_changelog" || true)"
release_date="${release_header#"## $workspace_version - "}"
[[ "$release_date" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] ||
    fail "$workspace_version changelog is not sealed with a release date"
[ "$(rg -c -F "## $workspace_version - $release_date" "$detailed_changelog")" -eq 1 ] ||
    fail "$workspace_version changelog header is duplicated"
if rg -F "## $workspace_version - Unreleased" "$detailed_changelog" >/dev/null; then
    fail "$workspace_version changelog still says Unreleased"
fi
head_subject="$(git -C "$ROOT" log -1 --format=%s HEAD)"
if [ "$head_subject" = "Release $workspace_version" ]; then
    validated_source="$(git -C "$ROOT" rev-parse HEAD^)"
else
    validated_source="$(git -C "$ROOT" rev-parse HEAD)"
fi
validation_marker="<!-- canic-release-validation: version=$workspace_version source=$validated_source date=$release_date -->"
[ "$(rg -c -F "$validation_marker" "$STATUS_DOCUMENT")" -eq 1 ] ||
    fail "current status is not bound to the exact validated source and release"

is_release_only_path() {
    case "$1" in
        Cargo.toml | Cargo.lock | scripts/dev/install_dev.sh | \
            scripts/ci/sync-release-surface-version.sh | \
            docs/status/current.md | "docs/changelog/$minor_line.md" | \
            */Cargo.toml)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

mapfile -t candidate_changes < <(
    git -C "$ROOT" diff --name-only "$validated_source" --
)
for changed_path in "${candidate_changes[@]}"; do
    is_release_only_path "$changed_path" ||
        fail "validated source is followed by non-release change: $changed_path"
done
while IFS= read -r untracked_path; do
    [ -z "$untracked_path" ] ||
        fail "release candidate contains untracked state: $untracked_path"
done < <(git -C "$ROOT" ls-files --others --exclude-standard)

metadata="$(cd "$ROOT" && cargo metadata --locked --offline --format-version 1 --no-deps)" ||
    fail "locked offline Cargo metadata is unavailable"
jq -e --arg version "$workspace_version" '
    .workspace_members as $members
    | [.packages[]
        | select(.id as $id | $members | index($id))
        | select(.version != $version)]
    | length == 0
' <<<"$metadata" >/dev/null ||
    fail "one or more workspace packages do not match $workspace_version"

expected_cli_version="CANIC_CLI_VERSION=\"\${CANIC_CLI_VERSION:-$workspace_version}\""
[ "$(rg -c -F "$expected_cli_version" "$INSTALL_DEV")" -eq 1 ] ||
    fail "install_dev.sh does not contain exactly one $workspace_version CLI default"

echo "release candidate guard passed ($workspace_version; validated source $validated_source; locked offline metadata)"
