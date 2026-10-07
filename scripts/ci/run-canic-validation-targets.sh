#!/usr/bin/env bash
set -euo pipefail

# Canic owns event identity and evidence defaults; Shared Tooling owns execution.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
export VALIDATION_LOG_DIR="${VALIDATION_LOG_DIR:-$ROOT/target/validation-runs}"
export VALIDATION_FAILURE_EVENT_PREFIX='[CANIC-TEST:E001]'

# Preserve test progress colors through the shared logger's output pipeline.
if [[ -t 1 && -z "${NO_COLOR:-}" && "${TERM:-dumb}" != "dumb" ]]; then
    export CANIC_TEST_COLOR="${CANIC_TEST_COLOR:-always}"
fi

exec bash "$ROOT/scripts/ci/run-validation-targets.sh" "$@"
