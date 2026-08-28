#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-validation-lane.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT

mkdir -p "$FIXTURE/bin" "$FIXTURE/scripts/ci"
cp "$ROOT/scripts/ci/run-release-validation-lane.sh" \
    "$FIXTURE/scripts/ci/run-release-validation-lane.sh"

printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "make %s\n" "$*" >>"$FIXTURE_EVENTS"' \
    'if [[ "$*" == *validate* ]]; then exit "${FAKE_VALIDATE_STATUS:-0}"; fi' \
    'exit 0' >"$FIXTURE/bin/make"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    '[[ "$*" = "rev-parse HEAD" ]] || exit 2' \
    'count=0' \
    '[[ ! -f "$FAKE_GIT_COUNT" ]] || read -r count <"$FAKE_GIT_COUNT"' \
    'count=$((count + 1))' \
    'printf "%s\n" "$count" >"$FAKE_GIT_COUNT"' \
    'if [[ "${FAKE_SOURCE_DRIFT:-0}" = 1 && "$count" -gt 1 ]]; then' \
    '    echo changed-head' \
    'else' \
    '    echo validated-head' \
    'fi' >"$FIXTURE/bin/git"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'exit "${FAKE_FAST_STATUS:-0}"' \
    >"$FIXTURE/scripts/ci/check-fast-patch-eligibility.sh"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "draft-preflight\n" >>"$FIXTURE_EVENTS"' \
    'exit "${FAKE_DRAFT_STATUS:-0}"' \
    >"$FIXTURE/scripts/ci/check-release-draft-ready.sh"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "bump=%s validated=%s head=%s kind=%s\n" "$1" "${CANIC_RELEASE_VALIDATED:-}" "${CANIC_RELEASE_VALIDATED_HEAD:-}" "${CANIC_RELEASE_VALIDATION_KIND:-}" >>"$FIXTURE_EVENTS"' \
    >"$FIXTURE/scripts/ci/bump-version.sh"
chmod +x "$FIXTURE/bin/make" "$FIXTURE/bin/git" \
    "$FIXTURE/scripts/ci/check-fast-patch-eligibility.sh" \
    "$FIXTURE/scripts/ci/check-release-draft-ready.sh" \
    "$FIXTURE/scripts/ci/bump-version.sh" \
    "$FIXTURE/scripts/ci/run-release-validation-lane.sh"

export FIXTURE_EVENTS="$FIXTURE/events"
export FAKE_GIT_COUNT="$FIXTURE/git-count"
export CANIC_RELEASE_RECEIPT_DIR="$FIXTURE/receipts"
PATH="$FIXTURE/bin:$PATH"
export PATH

reset_fixture() {
    rm -f "$FIXTURE_EVENTS" "$FAKE_GIT_COUNT"
}

assert_no_bump() {
    if [[ -f "$FIXTURE_EVENTS" ]] && rg -F 'bump=' "$FIXTURE_EVENTS" >/dev/null; then
        echo "release validation lane test failed: version mutation followed a failed gate" >&2
        exit 1
    fi
}

assert_no_validation() {
    if [[ -f "$FIXTURE_EVENTS" ]] && rg -F 'make --no-print-directory validate' "$FIXTURE_EVENTS" >/dev/null; then
        echo "release validation lane test failed: validation followed a failed draft preflight" >&2
        exit 1
    fi
}

reset_fixture
status=0
FAKE_DRAFT_STATUS=31 \
    bash "$FIXTURE/scripts/ci/run-release-validation-lane.sh" complete patch || status=$?
[[ "$status" -eq 31 ]] || {
    echo "release validation lane test failed: draft preflight failure status was $status" >&2
    exit 1
}
assert_no_validation
assert_no_bump

reset_fixture
status=0
FAKE_VALIDATE_STATUS=23 \
    bash "$FIXTURE/scripts/ci/run-release-validation-lane.sh" complete patch || status=$?
[[ "$status" -eq 23 ]] || {
    echo "release validation lane test failed: complete failure status was $status" >&2
    exit 1
}
assert_no_bump

reset_fixture
status=0
FAKE_FAST_STATUS=29 \
    bash "$FIXTURE/scripts/ci/run-release-validation-lane.sh" fast patch || status=$?
[[ "$status" -eq 29 ]] || {
    echo "release validation lane test failed: fast failure status was $status" >&2
    exit 1
}
assert_no_bump

reset_fixture
status=0
FAKE_SOURCE_DRIFT=1 \
    bash "$FIXTURE/scripts/ci/run-release-validation-lane.sh" complete minor || status=$?
[[ "$status" -eq 1 ]] || {
    echo "release validation lane test failed: source drift status was $status" >&2
    exit 1
}
assert_no_bump

reset_fixture
bash "$FIXTURE/scripts/ci/run-release-validation-lane.sh" complete major
rg -F 'bump=major validated=1 head=validated-head kind=complete' "$FIXTURE_EVENTS" >/dev/null || {
    echo "release validation lane test failed: successful complete gate did not bind its source" >&2
    exit 1
}

reset_fixture
bash "$FIXTURE/scripts/ci/run-release-validation-lane.sh" complete major
assert_no_validation
rg -F 'bump=major validated=1 head=validated-head kind=complete' "$FIXTURE_EVENTS" >/dev/null || {
    echo "release validation lane test failed: retained receipt did not resume version mutation" >&2
    exit 1
}

echo "release validation lane test passed"
