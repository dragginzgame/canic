#!/usr/bin/env bash
set -euo pipefail

# Generic logger mechanics are tested by the canonical upstream fixture.
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
unset VALIDATION_REPOSITORY_ROOT VALIDATION_RUNNER_SNAPSHOT_PATH
unset VALIDATION_LOG_DIR VALIDATION_FAILURE_LOG_DIR VALIDATION_RUNNER_DEPTH
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-validation-adapter.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$FIXTURE"; else printf "Validation adapter fixture retained: %s\n" "$FIXTURE" >&2; fi' EXIT
mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/make" "$FIXTURE/ci"
cp "$ROOT/scripts/ci/"{run-canic-validation-targets,run-validation-targets,check-make-execution,verify-file-checksum,ic-tool-pins}.sh "$FIXTURE/scripts/ci/"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$FIXTURE/"
cp "$ROOT/make/tools.mk" "$ROOT/make/execution.mk" "$ROOT/make/release.mk" "$ROOT/make/consumer.mk" "$ROOT/make/rust-format.mk" "$FIXTURE/make/"
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

# Exercise the real sequential recipes under parallel Make with inert gates.
cat >"$FIXTURE/gate" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
for target in "$@"; do
    printf '%s\n' "$target" >>"$ORDER_EVENTS"
    if [[ "$target" == clippy ]]; then
        exit "${ORDER_CLIPPY_STATUS:-0}"
    fi
done
SH
chmod +x "$FIXTURE/gate"
cat >>"$FIXTURE/Makefile" <<'MAKE'
VALIDATION_RUNNER = bash ./gate
tools-check:
	@echo tools-check >>"$(ORDER_EVENTS)"
ensure-clean:
	@echo ensure-clean >>"$(ORDER_EVENTS)"
clippy:
	@bash ./gate clippy
MAKE
export ORDER_EVENTS="$FIXTURE/order"
status=0
ORDER_CLIPPY_STATUS=23 make -j4 --no-print-directory -C "$FIXTURE" validate \
    >"$FIXTURE/order.log" 2>&1 || status=$?
[[ "$status" != 0 ]]
printf 'tools-check\ndependency-adoption-admission\nclippy\n' >"$FIXTURE/expected"
cmp "$FIXTURE/expected" "$ORDER_EVENTS"
: >"$ORDER_EVENTS"
status=0
ORDER_CLIPPY_STATUS=23 make -j4 --no-print-directory -C "$FIXTURE" package \
    >"$FIXTURE/package.log" 2>&1 || status=$?
[[ "$status" != 0 ]]
printf 'ensure-clean\nclippy\n' >"$FIXTURE/expected"
cmp "$FIXTURE/expected" "$ORDER_EVENTS"
: >"$ORDER_EVENTS"
make -j4 --no-print-directory -C "$FIXTURE" validate >"$FIXTURE/order.log" 2>&1
printf '%s\n' tools-check dependency-adoption-admission clippy dependency-adoption-fixtures fmt-check check-invariants shellcheck dependency-risk-gate dependency-risk-inventory-test \
    check control-plane-feature-gate test >"$FIXTURE/expected"
cmp "$FIXTURE/expected" "$ORDER_EVENTS"
# Hosted partitions consume the same groups, with deliberate cheap-first scheduling.
for target in ci-preflight ci-checks ci-security; do
    : > "$ORDER_EVENTS"
    make -j4 --no-print-directory -C "$FIXTURE" "$target" >"$FIXTURE/order.log" 2>&1
    case "$target" in
        ci-preflight) printf '%s\n' check-invariants shellcheck lint-workflows workspace-test-inventory-gate ;;
        ci-checks) printf '%s\n' fmt-check clippy control-plane-feature-gate ;;
        ci-security) printf '%s\n' dependency-risk-gate dependency-risk-inventory-test ;;
    esac >"$FIXTURE/expected"
    cmp "$FIXTURE/expected" "$ORDER_EVENTS"
done
: > "$ORDER_EVENTS"
if ORDER_CLIPPY_STATUS=23 make -j4 --no-print-directory -C "$FIXTURE" ci-checks >"$FIXTURE/order.log" 2>&1; then exit 1; fi
printf '%s\n' fmt-check clippy >"$FIXTURE/expected"
cmp "$FIXTURE/expected" "$ORDER_EVENTS"
# Changing a group's membership reaches both callers; there is no copied CI list.
for target in validate ci-security; do
    : > "$ORDER_EVENTS"
    make --no-print-directory -C "$FIXTURE" "$target" \
        'CANIC_QUALIFICATION_SECURITY=security-membership-probe' >"$FIXTURE/order.log" 2>&1
    [[ "$(grep -Fxc security-membership-probe "$ORDER_EVENTS")" == 1 ]]
    if grep -q '^dependency-risk-' "$ORDER_EVENTS"; then exit 1; fi
done
echo 'Canic validation policy, shared qualification groups and actual Make adapter pass'
