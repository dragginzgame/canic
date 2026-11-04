#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
[[ "${CANIC_TEST_PLAN_ONLY:-0}" == 0 ]] || { echo 'Release qualification requires actual test execution' >&2; exit 1; }
[[ "${RELEASE_SOURCE:?}" == "$(git rev-parse HEAD)" ]]
mkdir -p target/release-validation
attempt="$(mktemp -d "target/release-validation/$RELEASE_GROUP.XXXXXX")"
printf 'Canic release validation attempt: %s\n' "$attempt"
# A failed/interrupted fresh gate must invalidate the earlier successful pointer.
rm -f -- "target/release-validation/$RELEASE_GROUP/$RELEASE_SOURCE.json"
plan="$(perl scripts/release/validation-plan.pl "$attempt" "$@")"
VALIDATION_LOG_DIR="$attempt/logs" bash scripts/ci/run-canic-validation-targets.sh --plan "$plan"
perl scripts/release/validation-evidence.pl seal "$attempt"
