#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/ci/require-jq.sh
source "$(dirname "${BASH_SOURCE[0]}")/../ci/require-jq.sh"

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALL_DEV="$ROOT/scripts/dev/install_dev.sh"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"

fail() {
    echo "release candidate guard failed: $1" >&2
    exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo is unavailable"
require_jq
command -v rg >/dev/null 2>&1 || fail "rg is unavailable"

workspace_version="$(bash "$VERSION_READER")" ||
    fail "cargo-get could not read the root workspace version"
minor_line="${workspace_version%.*}"
detailed_changelog="$ROOT/docs/changelog/$minor_line.md"

[ -f "$detailed_changelog" ] ||
    fail "detailed changelog is missing for $workspace_version"
release_header="$(rg -m1 -F "## [$workspace_version] - " "$detailed_changelog" || true)"
release_date="${release_header#"## [$workspace_version] - "}"
[[ "$release_date" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] ||
    fail "$workspace_version changelog is not sealed with a release date"
[ "$(rg -c -F "## [$workspace_version] - $release_date" "$detailed_changelog")" -eq 1 ] ||
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
# Source validation belongs to the release lane. The editable handoff is not
# a publication receipt; this guard checks the sealed package surfaces.

is_release_only_path() {
    case "$1" in
        Cargo.toml | Cargo.lock | CHANGELOG.md | scripts/dev/install_dev.sh | \
            release-validation.json | "docs/changelog/$minor_line.md" | \
            */Cargo.toml)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

candidate_changes=()
while IFS= read -r value; do candidate_changes[${#candidate_changes[@]}]="$value"; done < <(
    git -C "$ROOT" diff --name-only "$validated_source" --
)
for changed_path in "${candidate_changes[@]}"; do
    is_release_only_path "$changed_path" ||
        fail "validated source is followed by non-release change: $changed_path"
    case "$changed_path" in
        Cargo.toml | */Cargo.toml | Cargo.lock)
            git -C "$ROOT" cat-file -e "$validated_source:$changed_path" ||
                fail "release version mutation added an unvalidated Cargo file: $changed_path"
            ;;
    esac
done
while IFS= read -r untracked_path; do
    [ -z "$untracked_path" ] ||
        fail "release candidate contains untracked state: $untracked_path"
done < <(git -C "$ROOT" ls-files --others --exclude-standard)

bash "$ROOT/scripts/ci/check-release-surface-content.sh" "$validated_source" "$workspace_version" "$release_date"

metadata="$(cd "$ROOT" && cargo metadata --locked --offline --format-version 1 --no-deps)" ||
    fail "locked offline Cargo metadata is unavailable"
# jq expressions use literal variable names.
# shellcheck disable=SC2016
"$JQ_BIN" -e --arg version "$workspace_version" '
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
