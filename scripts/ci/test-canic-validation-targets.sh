#!/usr/bin/env bash
set -euo pipefail

# Generic logger mechanics are tested by the canonical upstream fixture.
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
unset VALIDATION_REPOSITORY_ROOT VALIDATION_RUNNER_SNAPSHOT_PATH
unset VALIDATION_LOG_DIR VALIDATION_FAILURE_LOG_DIR VALIDATION_RUNNER_DEPTH
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-validation-adapter.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/make" "$FIXTURE/ci"
cp "$ROOT/scripts/ci/"{run-canic-validation-targets,run-validation-targets,check-make-execution,ic-tool-pins}.sh "$FIXTURE/scripts/ci/"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$FIXTURE/"
cp "$ROOT/make/tools.mk" "$FIXTURE/make/"
cp "$ROOT/ci/"{ic-tools.tsv,tool-versions.env} "$FIXTURE/ci/"

# Exercise the actual consumer Make adapter without running real gates.
cat >> "$FIXTURE/Makefile" <<'MAKE'
.PHONY: adapter-probe pass-probe failure-probe
adapter-probe:
	$(VALIDATION_RUNNER) pass-probe failure-probe
pass-probe:
	@echo canic-pass-marker
failure-probe:
	@echo '[CANIC-TEST:E001] [SUITE] FAIL canic-probe-marker'
	@exit 6
MAKE
status=0
make --no-print-directory -C "$FIXTURE" adapter-probe > "$FIXTURE/output.log" 2>&1 || status=$?
[[ "$status" != 0 ]]
rg -F '[ERR:failure-probe] [CANIC-TEST:E001] [SUITE] FAIL canic-probe-marker' "$FIXTURE/output.log" > /dev/null
rg -F '[ERR:failure-probe] [CANIC-TEST:E001] [SUITE] FAIL canic-probe-marker' "$FIXTURE/target/validation-failures/latest-errors.log" > /dev/null
timings="$(rg --files "$FIXTURE/target/validation-runs" | rg '/timings.tsv$')"
pass_log="$(awk -F '\t' '$1 == "pass-probe" && $2 == "PASS" { print $4 }' "$timings")"
rg -F canic-pass-marker "$pass_log" > /dev/null
echo 'Canic validation policy and actual Make adapter pass'
