#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-recipes.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/scripts/ci"
cp "$ROOT/tool-versions.env" "$fixture/"
mkdir -p "$fixture/ci" "$fixture/scripts/ci"
cp "$ROOT/ci/ic-tools.tsv" "$ROOT/ci/tool-versions.env" "$fixture/ci/"
mkdir -p "$fixture/make"
cp "$ROOT/make/tools.mk" "$ROOT/make/execution.mk" "$ROOT/make/release.mk" "$ROOT/make/consumer.mk" "$ROOT/make/rust-format.mk" "$fixture/make/"
cp "$ROOT/scripts/ci/check-make-execution.sh" "$fixture/scripts/ci/"
cp "$ROOT/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
cat >"$fixture/bin/record" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == */check-make-execution.sh ]]; then exec "$CANIC_RECIPE_TEST_BASH" "$@"; fi
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
export CANIC_RECIPE_TEST_BASH="$real_bash"
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
    PATH="$fixture/bin:$PATH" "$real_make" -j4 --no-print-directory -f "$ROOT/Makefile" \
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
# Validate is the complete release-verify path. Sequential recipes must also
# stop correctly when the caller enables parallel Make.
run_case validate success
printf '%s\n' '--no-print-directory tools-check' \
    'scripts/ci/run-canic-validation-targets.sh dependency-adoption-admission' \
    'scripts/ci/run-canic-validation-targets.sh clippy' \
    'scripts/ci/run-canic-validation-targets.sh dependency-adoption-fixtures' \
    'scripts/ci/run-canic-validation-targets.sh fmt-check check-invariants dependency-risk-gate shellcheck' \
    'scripts/ci/run-canic-validation-targets.sh check' \
    'scripts/ci/run-canic-validation-targets.sh control-plane-feature-gate' \
    'scripts/ci/run-canic-validation-targets.sh test' > "$fixture/expected"
cmp "$fixture/expected" "$EVENTS"
for gate in dependency-adoption-admission clippy dependency-adoption-fixtures; do
    FAIL_AT="scripts/ci/run-canic-validation-targets.sh $gate" run_case validate failure
    [[ "$(tail -1 "$EVENTS")" == "scripts/ci/run-canic-validation-targets.sh $gate" ]]
done
echo 'release Make recipes preserve ordering and failure boundaries (no Git effects)'
