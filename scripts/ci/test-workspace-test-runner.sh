#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-workspace-runner-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/scripts/ci" "$fixture/bin"
cp "$ROOT/scripts/ci/run-workspace-tests.sh" \
    "$ROOT/scripts/ci/workspace-scope.sh" \
    "$ROOT/scripts/ci/list-internal-native-tests.sh" \
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
if [[ "$1" == run ]]; then
    [[ "$*" == 'run --locked --offline -p canic-testing-internal --example verify_embedded_root' ]]
    [[ ! -e "$CANIC_TEST_SCRATCH/server.pid" ]]
    printf 'preflight\tembedded-root\t1\n' >> "$RUNNER_TEST_TRACE"
    if [[ "$RUNNER_TEST_FAIL_STAGE" == preflight/embedded-root ]]; then
        echo 'error: embedded fixture differs from current source' >&2
        exit 1
    fi
    exit 0
fi
[[ "$1" == test ]]
if [[ " $* " == *' --workspace '* ]]; then
    [[ " $* " == *' --exclude canic-icydb-lifecycle-schema '* ]]
    [[ " $* " == *' --exclude canic_icydb_lifecycle_probe '* ]]
fi
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
    [[ "$argument" == --no-run || "$argument" == --no-fail-fast ]] || graph_args+=("$argument")
done
case " $* " in
    *' pic::governed_suite::governed_pocketic_inventory_preserves_recovery_prefix_and_journey_suffix '*) stage=native-inventory ;;
    *' pic::workers::tests:: '*) stage=native-internal ;;
    *' --features local-fleet '*) stage=native-host ;;
    *' --doc '*) stage=documentation ;;
esac
[[ " $* " != *' --list '* ]] || phase=list
if [[ -n "$target" && " $* " == *' -p canic-tests '* ]]; then
    stage="$(awk -F '\t' -v target="$target" '$2 == target { print $5 }' scripts/ci/workspace-test-inventory.tsv)"
    if [[ "$stage" == external-composition ]]; then
        [[ " $* " == *' --features external-composition '* ]]
    else
        [[ " $* " != *'external-composition'* ]]
    fi
fi
if [[ "$phase" == compile || "$stage" == ordinary || "$stage" == native-* || "$stage" == documentation ]]; then
    [[ ! -e "$CANIC_TEST_SCRATCH/server.pid" ]]
elif [[ "$phase" == execute ]]; then
    [[ -e "$CANIC_TEST_SCRATCH/server.pid" ]]
fi
printf '%s\n' "${graph_args[@]}" > "$CANIC_TEST_SCRATCH/$phase-$stage.args"
printf '%s\t%s\t%s\n' "$phase" "$stage" "$fail_fast" >> "$RUNNER_TEST_TRACE"
if [[ "$phase" == list ]]; then
    [[ "$stage" != "${RUNNER_TEST_EMPTY_STAGE:-}" ]] || exit 0
    if [[ "$stage" == internal || "$stage" == host ]]; then
        [[ " $* " == *' --ignored '* ]]
    fi
    echo "${RUNNER_TEST_LIST_IDENTITY:-fixture::selected}: test"
    [[ "${RUNNER_TEST_DUPLICATE:-0}" == 0 ]] || echo "${RUNNER_TEST_LIST_IDENTITY}: test"
    exit 0
fi
# Successful tests can contain rejected requests; only the command outcome
# decides whether these diagnostics belong in the console.
printf '[CANIC-REQUEST] %s/%s succeeded=false\n' "$phase" "$stage"
printf '[CANIC-OBSERVATION] %s/%s\n' "$phase" "$stage" >&2
printf '[CANIC-TIMING] %s/%s\n' "$phase" "$stage" >&2
printf '[CANIC-CACHE] %s/%s\n' "$phase" "$stage" >&2
printf '[FLEET-MEASURE] %s/%s stdout\n' "$phase" "$stage"
printf '[FLEET-MEASURE] %s/%s stderr\n' "$phase" "$stage" >&2
printf 'fixture progress %s/%s\n' "$phase" "$stage"
fails=0
case " $RUNNER_TEST_FAIL_STAGE " in *" $phase/$stage "*) fails=1 ;; esac
if [[ "$fails" -eq 1 ]]; then
    for ((index=0; index<120; index++)); do
        if ((index % 3 == 0)); then
            printf '[CANIC-REQUEST] failure-context-%s\n' "$index" >&2
        elif ((index % 3 == 1)); then
            printf '[CANIC-CACHE] failure-context-%s\n' "$index" >&2
        else
            printf '[FLEET-MEASURE] failure-context-%s\n' "$index" >&2
        fi
    done
    echo 'error: fixture test failed' >&2
    echo "assertion failed: $stage fixture invariant" >&2
fi
[[ "$fails" -eq 0 ]] || exit 101
if [[ "$phase" == execute ]]; then
    # Full workspace Host selection also contains unrelated empty harnesses.
    echo 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s'
    if [[ "$stage" != "${RUNNER_TEST_ZERO_STAGE:-}" ]]; then
        echo 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s'
    fi
fi
SH
chmod +x "$fixture/bin/"*

serial_stages=(internal host runtime blob-storage payload-limits)
for mode in full pocketic; do
    stages=(preflight/embedded-root)
    if [[ "$mode" == full ]]; then
        stages+=(execute/ordinary list/native-internal execute/native-internal list/native-host execute/native-host execute/documentation)
    fi
    for stage in "${serial_stages[@]}"; do stages+=("compile/$stage" "list/$stage"); done
    for stage in "${serial_stages[@]}"; do stages+=("execute/$stage"); done
    for failure in "${stages[@]}" none; do
        [[ "$failure" != list/* && "$failure" != preflight/* ]] || continue
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
            [[ "$selected" != execute/* ]] || fail_fast=0
            printf '%s\t%s\t%s\n' "${selected%/*}" "${selected#*/}" "$fail_fast"
            if [[ "$selected" == "$failure" && "$failure" == compile/* ]]; then break; fi
            if [[ "$selected" == execute/documentation &&
                ("$failure" == execute/ordinary || "$failure" == execute/native-* || "$failure" == execute/documentation) ]]; then break; fi
        done > "$scratch/expected.tsv"
        diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
        logs="$fixture/target/test-runs"
        # The original streams survive even when the successful console is quiet.
        rg -q '\[CANIC-REQUEST\].*succeeded=false' "$logs"
        rg -q '\[CANIC-OBSERVATION\]' "$logs"
        rg -q '\[CANIC-TIMING\]' "$logs"
        rg -q '\[CANIC-CACHE\]' "$logs"
        rg -q '\[FLEET-MEASURE\].*stdout' "$logs"
        rg -q '\[FLEET-MEASURE\].*stderr' "$logs"
        rg -q 'fixture progress' "$scratch/output.log"
        if [[ "$failure" == none ]]; then
            if rg -q '\[(CANIC-(REQUEST|OBSERVATION|TIMING|CACHE)|FLEET-MEASURE)\]' "$scratch/output.log"; then exit 1; fi
        else
            rg -q '^error: fixture test failed$' "$scratch/output.log"
            rg -q '^\[FLEET-MEASURE\] failure-context-119$' "$scratch/output.log"
            rg -q '^\[CANIC-CACHE\] failure-context-118$' "$scratch/output.log"
            if rg -q '^\[CANIC-REQUEST\] failure-context-0$' "$scratch/output.log"; then exit 1; fi
            [[ "$(rg -c '\[(CANIC-(REQUEST|OBSERVATION|TIMING|CACHE)|FLEET-MEASURE)\]' "$scratch/output.log")" -eq 100 ]]
            rg -q '^\[CANIC-REQUEST\] failure-context-0$' "$logs"
        fi
        rm -rf "$logs"
        if [[ "$failure" == execute/ordinary || "$failure" == execute/native-* || "$failure" == execute/documentation || "$failure" == compile/* ]]; then
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

# Stale producer evidence must stop before any suite or server, in both broad
# test modes. Narrow modes below still select only their requested behavior.
for mode in full pocketic; do
    scratch="$fixture/stale-embedded-$mode"
    mkdir -p "$scratch"
    status=0
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=preflight/embedded-root \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" "$mode" > "$scratch/output.log" 2>&1 || status=$?
    [[ "$status" -ne 0 && ! -e "$scratch/server.pid" ]]
    printf 'preflight\tembedded-root\t1\n' > "$scratch/expected.tsv"
    diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
    rg -q '^EMBEDDED FIXTURE PREFLIGHT FAILED:' "$scratch/output.log"
    rg -q '^error: embedded fixture differs from current source$' "$scratch/output.log"
done

# A missing selector fails admission even when Cargo exits successfully. Native
# groups still finish their independent checks; serial selectors stop before a server.
for stage in native-internal native-host "${serial_stages[@]}"; do
    scratch="$fixture/empty-$stage"
    mkdir -p "$scratch"
    status=0
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
        RUNNER_TEST_EMPTY_STAGE="$stage" \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" full > "$scratch/output.log" 2>&1 || status=$?
    [[ "$status" -ne 0 && ! -e "$scratch/server.pid" ]]
    rg -q 'test selector resolved to zero tests' "$scratch/output.log"
    if awk -F '\t' -v stage="$stage" '$1 == "execute" && $2 == stage { found=1 } END { exit !found }' "$scratch/trace.tsv"; then exit 1; fi
done

for stage in native-internal internal host; do
    scratch="$fixture/zero-executed-$stage"
    mkdir -p "$scratch"
    status=0
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
        RUNNER_TEST_ZERO_STAGE="$stage" \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" full > "$scratch/output.log" 2>&1 || status=$?
    [[ "$status" -ne 0 ]]
    rg -q 'selected test command completed without executing a test' "$scratch/output.log"
done

exact='pic::governed_suite::governed_internal_pocketic_suite'
for scenario in correct wrong duplicate; do
    scratch="$fixture/exact-$scenario"
    mkdir -p "$scratch"
    identity="$exact" duplicate=0 status=0
    [[ "$scenario" != wrong ]] || identity=wrong::case
    [[ "$scenario" != duplicate ]] || duplicate=1
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
        RUNNER_TEST_LIST_IDENTITY="$identity" RUNNER_TEST_DUPLICATE="$duplicate" \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" targeted-pocketic "$exact" > "$scratch/output.log" 2>&1 || status=$?
    if [[ "$scenario" == correct ]]; then
        [[ "$status" -eq 0 ]]
    else
        [[ "$status" -ne 0 ]]
        rg -q 'exact PocketIC selector must resolve to one test' "$scratch/output.log"
    fi
done

# An omitted registration must fail even when an individual journey was selected.
# Catch it before server startup; the zero-tests case also must not pass silently.
for scenario in failure empty; do
    scratch="$fixture/targeted-inventory-$scenario"
    mkdir -p "$scratch"
    failure=none zero='' status=0
    if [[ "$scenario" == failure ]]; then
        failure=execute/native-inventory
    else
        zero=native-inventory
    fi
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE="$failure" \
        RUNNER_TEST_ZERO_STAGE="$zero" \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" targeted-pocketic "$exact" \
        > "$scratch/output.log" 2>&1 || status=$?
    [[ "$status" -ne 0 && ! -e "$scratch/server.pid" ]]
    printf 'execute\tnative-inventory\t0\n' > "$scratch/expected.tsv"
    diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
    rg -q '^POCKETIC INVENTORY PREFLIGHT FAILED:' "$scratch/output.log"
done

scratch="$fixture/multiple-failures"
mkdir -p "$scratch"
status=0
CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
    POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
    RUNNER_TEST_TRACE="$scratch/trace.tsv" \
    RUNNER_TEST_FAIL_STAGE='execute/internal execute/host execute/runtime' \
    bash "$fixture/scripts/ci/run-workspace-tests.sh" full > "$scratch/output.log" 2>&1 || status=$?
[[ "$status" -ne 0 ]]
for stage in internal host runtime; do
    rg -q "assertion failed: $stage fixture invariant" "$scratch/output.log"
done
rg -q $'^execute\tpayload-limits\t0$' "$scratch/trace.tsv"

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
        printf 'list\tpayload-limits\t1\nexecute\tpayload-limits\t0\n' > "$scratch/expected.tsv"
        read -r server_pid < "$scratch/server.pid"
        if kill -0 "$server_pid" 2>/dev/null; then
            echo "targeted runner left its fixture server running" >&2
            exit 1
        fi
    else
        printf 'execute\tordinary\t0\n' > "$scratch/expected.tsv"
        if [[ "$mode" == ordinary ]]; then
            printf 'list\tnative-internal\t1\nexecute\tnative-internal\t0\nlist\tnative-host\t1\nexecute\tnative-host\t0\nexecute\tdocumentation\t0\n' >> "$scratch/expected.tsv"
        fi
        [[ ! -e "$scratch/server.pid" ]]
    fi
    diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
    if rg -q '\[(CANIC-(REQUEST|OBSERVATION|TIMING|CACHE)|FLEET-MEASURE)\]' "$scratch/output.log"; then exit 1; fi
done
echo 'workspace test runner barriers, selectors, failure ordering, quiet output, retained diagnostics and cleanup passed'

# Explicit external selection enables its optional dependency before list/run.
scratch="$fixture/external-composition"
mkdir -p "$scratch"
CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
    POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
    RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
    bash "$fixture/scripts/ci/run-workspace-tests.sh" targeted-pocketic icydb_lifecycle_composition \
    > "$scratch/output.log" 2>&1
printf 'list\texternal-composition\t1\nexecute\texternal-composition\t0\n' > "$scratch/expected.tsv"
diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"

# Cache accounting distinguishes misses/uncacheable outputs from infrastructure
# errors, rejects partial stats and never reports a negative delta after reset.
cat > "$fixture/bin/sccache" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == --show-stats ]]
[[ "$RUNNER_CACHE_SCENARIO" != unavailable ]] || exit 2
if [[ "$RUNNER_CACHE_SCENARIO" == malformed ]]; then
    echo 'Compile requests 10'
    exit 0
fi
count=0
[[ ! -e "$CANIC_TEST_SCRATCH/cache-count" ]] || read -r count < "$CANIC_TEST_SCRATCH/cache-count"
echo "$((count + 1))" > "$CANIC_TEST_SCRATCH/cache-count"
if [[ "$RUNNER_CACHE_SCENARIO" == reset && "$count" -gt 0 ]]; then count=-1; fi
printf 'Compile requests %s\nCache hits %s\nCache misses %s\n' "$((20 + count * 10))" "$((5 + count * 2))" "$((5 + count * 3))"
printf 'Non-cacheable calls %s\nCache errors %s\n' "$((10 + count * 5))" "$((2 + count))"
printf 'Cache read errors 0\nCache write errors 0\nCache timeouts 0\n'
SH
chmod +x "$fixture/bin/sccache"
for scenario in healthy reset malformed unavailable; do
    scratch="$fixture/cache-$scenario"
    mkdir -p "$scratch"
    CI=0 RUSTC_WRAPPER="$fixture/bin/sccache" CANIC_TEST_PLAN_ONLY=0 \
        CANIC_TEST_SCRATCH="$scratch" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
        RUNNER_CACHE_SCENARIO="$scenario" \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" ordinary > "$scratch/output.log" 2>&1
    case "$scenario" in
        healthy) rg -q 'compiler cache delta: requests=10 hits=2 misses=3 uncacheable=5 cache_errors=1' "$scratch/output.log" ;;
        reset) rg -q 'compiler cache delta: unavailable' "$scratch/output.log" ;;
        *) rg -q 'compiler cache observation: unavailable' "$scratch/output.log" ;;
    esac
done
echo 'compiler cache observation tests passed'

# An empty filter means "all tests" to libtest. Never admit stateful tests through
# an empty, partial or failed native-selector producer.
for scenario in empty blank failed; do
    scratch="$fixture/native-selector-$scenario"
    mkdir -p "$scratch"
    case "$scenario" in
        empty) printf '#!/usr/bin/env bash\nexit 0\n' > "$fixture/scripts/ci/list-internal-native-tests.sh" ;;
        blank) printf '#!/usr/bin/env bash\nprintf "pic::workers::tests::\\n\\npic::governed_suite::\\n"\n' > "$fixture/scripts/ci/list-internal-native-tests.sh" ;;
        failed) printf '#!/usr/bin/env bash\necho "pic::workers::tests::"\nexit 1\n' > "$fixture/scripts/ci/list-internal-native-tests.sh" ;;
    esac
    status=0
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 CANIC_TEST_SCRATCH="$scratch" \
        POCKET_IC_BIN="$fixture/bin/pocket-ic" PATH="$fixture/bin:$PATH" \
        RUNNER_TEST_TRACE="$scratch/trace.tsv" RUNNER_TEST_FAIL_STAGE=none \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" ordinary > "$scratch/output.log" 2>&1 || status=$?
    [[ "$status" -ne 0 ]]
    if rg -q 'native-internal' "$scratch/trace.tsv"; then exit 1; fi
done
echo 'empty and failed native selector producers reject before execution'
