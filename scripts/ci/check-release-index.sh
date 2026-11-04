#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
roster="$(mktemp "${TMPDIR:-/tmp}/canic-release-index.XXXXXX")"
trap 'rm -f -- "$roster"' EXIT
bash scripts/release/metadata.sh files > "$roster"
[[ -n "$(git diff --cached --name-only)" ]] || { echo 'No staged release files' >&2; exit 1; }
[[ -z "$(git diff --cached --name-only --diff-filter=D)" ]] || exit 1
while IFS= read -r -d '' staged; do
    admitted=false
    while IFS= read -r -d '' path; do [[ "$staged" != "$path" ]] || admitted=true; done < "$roster"
    [[ "$admitted" == true ]] || { echo "Unexpected staged path: $staged" >&2; exit 1; }
    git diff --quiet -- "$staged" || { echo "Partially staged release file: $staged" >&2; exit 1; }
done < <(git diff --cached --name-only -z)
echo 'Release index contains complete selected-group files'
