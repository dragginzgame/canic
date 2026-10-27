#!/usr/bin/env bash
set -euo pipefail

# Both delivery paths stage the adapter's one governed, NUL-delimited roster.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export RELEASE_VERSION
RELEASE_VERSION="$(bash scripts/ci/read-workspace-version.sh)"
files="$(mktemp "${TMPDIR:-/tmp}/canic-release-stage.XXXXXX")"
trap 'rm -f "$files"' EXIT
bash scripts/release/adapter.sh files > "$files"
[[ -s "$files" ]] || { echo 'release file roster is empty' >&2; exit 1; }
git --literal-pathspecs add --pathspec-from-file="$files" --pathspec-file-nul
