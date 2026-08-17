#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALL_DEV="$ROOT/scripts/dev/install_dev.sh"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"

fail() {
    echo "release candidate guard failed: $1" >&2
    exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo is unavailable"
command -v jq >/dev/null 2>&1 || fail "jq is unavailable"
command -v rg >/dev/null 2>&1 || fail "rg is unavailable"

workspace_version="$(bash "$VERSION_READER")" ||
    fail "cargo-get could not read the root workspace version"

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

echo "release candidate guard passed ($workspace_version; locked offline metadata)"
