#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-workspace-runner-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/bin"
cp "$ROOT/scripts/ci/run-workspace-tests.sh" \
    "$ROOT/scripts/ci/workspace-test-inventory.tsv" "$fixture/scripts/ci/"
cp "$ROOT/tool-versions.env" "$fixture/"

# Exercise the real runner's ordering, exit and cleanup boundaries without
# building crates, opening sockets or executing canisters.
for prerequisite in check-workspace-test-inventory check-pocketic-version-alignment; do
    printf '#!/usr/bin/env bash\nexit 0\n' > "$fixture/scripts/ci/$prerequisite.sh"
done
printf 'use_native_test_icp() { :; }\n' > "$fixture/scripts/ci/native-icp-lib.sh"
cat > "$fixture/bin/pocket-ic" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$$" > "$CANIC_TEST_SCRATCH/server.pid"
while [[ "$#" -gt 0 ]]; do
    if [[ "$1" == --port-file ]]; then
        printf '12345\n' > "$2"
        break
    fi
    shift
done
exec sleep 60
SH
cat > "$fixture/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" != fetch ]] || exit 0
[[ "$1" == test ]]
stage=ordinary
fail_fast=1
phase=execute
target=''
previous=''
graph_args=()
for argument in "$@"; do
    [[ "$argument" != -- ]] || break
    case "$argument" in
        --no-fail-fast) fail_fast=0 ;;
        --no-run) phase=compile ;;
        governed_pocketic_) stage=host ;;
        governed-pocketic-tests) stage=internal ;;
    esac
    [[ "$previous" != --test ]] || target="$argument"
    previous="$argument"
    [[ "$argument" == --no-run ]] || graph_args+=("$argument")
done
if [[ -n "$target" && " $* " == *' -p canic-tests '* ]]; then
    stage="$(awk -F '\t' -v target="$target" '$2 == target { print $5 }' scripts/ci/workspace-test-inventory.tsv)"
fi
if [[ "$phase" == compile || "$stage" == ordinary ]]; then
    [[ ! -e "$CANIC_TEST_SCRATCH/server.pid" ]]
else
    [[ -e "$CANIC_TEST_SCRATCH/server.pid" ]]
fi
printf '%s\n' "${graph_args[@]}" > "$CANIC_TEST_SCRATCH/$phase-$stage.args"
printf '%s\t%s\t%s\n' "$phase" "$stage" "$fail_fast" >> "$RUNNER_TEST_TRACE"
[[ "$phase/$stage" != "$RUNNER_TEST_FAIL_STAGE" ]] || exit 101
SH
chmod +x "$fixture/bin/"*

serial_stages=(internal host runtime blob-storage payload-limits)
for mode in full pocketic; do
    stages=()
    [[ "$mode" != full ]] || stages+=(execute/ordinary)
    for stage in "${serial_stages[@]}"; do stages+=("compile/$stage"); done
    for stage in "${serial_stages[@]}"; do stages+=("execute/$stage"); done
    for failure in "${stages[@]}" none; do
        scratch="$fixture/$mode/${failure//\//-}"
        mkdir -p "$scratch"
        status=0
        CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 GITHUB_STEP_SUMMARY="$scratch/summary.md" \
            CANIC_TEST_SCRATCH="$scratch" POCKET_IC_BIN="$fixture/bin/pocket-ic" \
            PATH="$fixture/bin:$PATH" RUNNER_TEST_TRACE="$scratch/trace.tsv" \
            RUNNER_TEST_FAIL_STAGE="$failure" \
            bash "$fixture/scripts/ci/run-workspace-tests.sh" "$mode" > "$scratch/output.log" 2>&1 || status=$?
        if [[ "$failure" == none ]]; then
            [[ "$status" -eq 0 ]]
        else
            [[ "$status" -ne 0 ]]
        fi
        for selected in "${stages[@]}"; do
            fail_fast=1
            [[ "$selected" != execute/ordinary ]] || fail_fast=0
            printf '%s\t%s\t%s\n' "${selected%/*}" "${selected#*/}" "$fail_fast"
            [[ "$selected" != "$failure" ]] || break
        done > "$scratch/expected.tsv"
        diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
        if [[ "$failure" == execute/ordinary || "$failure" == compile/* ]]; then
            [[ ! -e "$scratch/server.pid" ]]
        else
            read -r server_pid < "$scratch/server.pid"
            if kill -0 "$server_pid" 2>/dev/null; then
                echo "runner left its fixture server running after $failure" >&2
                exit 1
            fi
        fi
        if [[ "$failure" == none ]]; then
            for stage in "${serial_stages[@]}"; do
                diff -u "$scratch/compile-$stage.args" "$scratch/execute-$stage.args"
            done
        fi
    done
done

# Narrow modes must not inherit the complete serial compilation barrier.
for mode in ordinary fast targeted-pocketic; do
    scratch="$fixture/$mode"
    mkdir -p "$scratch"
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" "$mode" pic_ingress_payload_limits \
        > "$scratch/output.log" 2>&1
    if [[ "$mode" == targeted-pocketic ]]; then
        printf 'execute\tpayload-limits\t1\n' > "$scratch/expected.tsv"
        read -r server_pid < "$scratch/server.pid"
        if kill -0 "$server_pid" 2>/dev/null; then
            echo "targeted runner left its fixture server running" >&2
            exit 1
        fi
    else
        printf 'execute\tordinary\t0\n' > "$scratch/expected.tsv"
        [[ ! -e "$scratch/server.pid" ]]
    fi
    diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
done
echo 'workspace test runner compilation barriers, selectors, failure ordering and cleanup passed'
