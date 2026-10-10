#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/standard-release-entry.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/ci" "$fixture/make" "$fixture/scripts/ci"
cp "$root/Makefile" "$root/tool-versions.env" "$fixture/"
cp "$root/ci/tool-versions.env" "$root/ci/ic-tools.tsv" "$fixture/ci/"
cp "$root/make/tools.mk" "$root/make/execution.mk" "$root/make/release.mk" "$fixture/make/"
cp "$root/scripts/ci/check-make-execution.sh" "$fixture/scripts/ci/"
cp "$root/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
real_bash="$(command -v bash)"
printf '#!%s\n' "$real_bash" > "$fixture/bin/bash"
cat >> "$fixture/bin/bash" <<'STUB'
set -euo pipefail
if [[ "$1" == */check-make-execution.sh ]]; then exec "$CANIC_RECIPE_TEST_BASH" "$@"; fi
printf '%s\n' "$*" >> "$EVENTS"
[[ "${FAIL_RUNNER:-0}" == 0 ]]
STUB
chmod +x "$fixture/bin/bash"
export EVENTS="$fixture/events"
export CANIC_RECIPE_TEST_BASH="$real_bash"
cd "$fixture"
real_make="$(command -v make)"
for kind in patch minor major; do
    for fail in 0 1; do
        : > "$EVENTS"
        status=0
        PATH="$fixture/bin:$PATH" FAIL_RUNNER="$fail" "$real_make" --no-print-directory \
            "release-$kind" RELEASE_REMOTE=review RELEASE_BRANCH=release-review \
            > "$fixture/output" 2>&1 || status=$?
        if [[ "$fail" == 0 ]]; then [[ "$status" == 0 ]]; else [[ "$status" != 0 ]]; fi
        printf '%s\n' "$fixture/scripts/ci/run-release.sh $kind review release-review" > "$fixture/expected"
        cmp "$fixture/expected" "$EVENTS"
    done
done
: > "$EVENTS"
PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory \
    release-resume VERSION=0.1.1 RELEASE_REMOTE=review RELEASE_BRANCH=release-review > "$fixture/output" 2>&1
printf '%s\n' "$fixture/scripts/ci/run-release.sh resume 0.1.1 review release-review" > "$fixture/expected"
cmp "$fixture/expected" "$EVENTS"
: > "$EVENTS"
if PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory \
    release-patch release-minor > "$fixture/output" 2>&1; then exit 1; fi
[[ ! -s "$EVENTS" ]]
echo 'standard release Make adapters passed (command stubs; no Git effects)'
