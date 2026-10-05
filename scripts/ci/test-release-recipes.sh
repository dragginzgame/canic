#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-recipes.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/ci"
cp "$ROOT/tool-versions.env" "$fixture/"
cat >"$fixture/bin/record" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$EVENTS"
if [[ "$*" == scripts/ci/push-release.sh ]]; then
    [[ "${CANIC_RELEASE_PUSH_READY:-}" == 1 ]] || exit 28
fi
[[ "$*" != "${FAIL_AT:-}" ]] || exit 27
SH
cp "$fixture/bin/record" "$fixture/bin/bash"
cp "$fixture/bin/record" "$fixture/bin/make"
cat >"$fixture/scripts/ci/check-release-index.sh" <<'SH'
#!/usr/bin/env bash
exec record index
SH
# Use the caller's real Bash to execute the recording stubs. No release helper
# or Git command runs; Make's actual recipes own all sequencing in this fixture.
real_bash="$(command -v bash)"
for script in "$fixture/bin/record" "$fixture/bin/bash" "$fixture/bin/make" "$fixture/scripts/ci/check-release-index.sh"; do
    sed "1c\\#!$real_bash" "$script" >"$fixture/rewritten"
    cp "$fixture/rewritten" "$script"
    chmod +x "$script"
done
export EVENTS="$fixture/events"
cd "$fixture"
# Resolve Make before overriding PATH for the recipes.
real_make="$(command -v make)"
run_case() {
    local target="$1" expected="$2" status=0
    : >"$EVENTS"
    PATH="$fixture/bin:$PATH" "$real_make" --no-print-directory -f "$ROOT/Makefile" \
        MAKE="$fixture/bin/make" "$target" >"$fixture/output" 2>&1 || status=$?
    if [[ "$expected" == success ]]; then [[ "$status" == 0 ]]; else [[ "$status" != 0 ]]; fi
}
# The canonical runner fixtures own phase ordering for the standard commands.
bash "$ROOT/scripts/release/test-standard-release.sh"
run_case release-patch-fast success
printf '%s\n' patch-fast release-stage release-commit release-push >"$fixture/expected"
cmp "$fixture/expected" "$EVENTS"
FAIL_AT=release-commit run_case release-patch-fast failure
printf '%s\n' patch-fast release-stage release-commit >"$fixture/expected"
cmp "$fixture/expected" "$EVENTS"
run_case release-commit success
printf '%s\n' index '--no-print-directory release-candidate' 'scripts/ci/commit-release.sh' >"$fixture/expected"
cmp "$fixture/expected" "$EVENTS"
FAIL_AT=index run_case release-commit failure
[[ "$(cat "$EVENTS")" == index ]]
FAIL_AT='--no-print-directory release-candidate' run_case release-commit failure
[[ "$(tail -1 "$EVENTS")" == '--no-print-directory release-candidate' ]]
run_case release-push success
printf '%s\n' scripts/ci/check-release-push-ready.sh scripts/ci/push-release.sh >"$fixture/expected"
cmp "$fixture/expected" "$EVENTS"
FAIL_AT=scripts/ci/check-release-push-ready.sh run_case release-push failure
[[ "$(cat "$EVENTS")" == scripts/ci/check-release-push-ready.sh ]]
echo 'release Make recipes preserve ordering and failure boundaries (no Git effects)'
