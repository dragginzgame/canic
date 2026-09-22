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
target=''
previous=''
for argument in "$@"; do
    case "$argument" in
        --no-fail-fast) fail_fast=0 ;;
        --test-threads=1) stage=host ;;
    esac
    [[ "$previous" != --test ]] || target="$argument"
    previous="$argument"
done
if [[ "$stage" == host ]]; then
    for argument in "$@"; do
        if [[ "$argument" == governed-pocketic-tests ]]; then
            stage=internal
        fi
    done
    if [[ -n "$target" && " $* " == *' -p canic-tests '* ]]; then
        stage="$(awk -F '\t' -v target="$target" '$2 == target { print $5 }' scripts/ci/workspace-test-inventory.tsv)"
    fi
fi
printf '%s\t%s\n' "$stage" "$fail_fast" >> "$RUNNER_TEST_TRACE"
[[ "$stage" != "$RUNNER_TEST_FAIL_STAGE" ]] || exit 101
SH
chmod +x "$fixture/bin/"*

stages=(ordinary internal host runtime blob-storage payload-limits)
for failure in "${stages[@]}" none; do
    scratch="$fixture/$failure"
    mkdir -p "$scratch"
    status=0
    CI=0 RUSTC_WRAPPER='' CANIC_TEST_PLAN_ONLY=0 GITHUB_STEP_SUMMARY="$scratch/summary.md" \
        CANIC_TEST_SCRATCH="$scratch" POCKET_IC_BIN="$fixture/bin/pocket-ic" \
        PATH="$fixture/bin:$PATH" RUNNER_TEST_TRACE="$scratch/trace.tsv" \
        RUNNER_TEST_FAIL_STAGE="$failure" \
        bash "$fixture/scripts/ci/run-workspace-tests.sh" full > "$scratch/output.log" 2>&1 || status=$?
    if [[ "$failure" == none ]]; then
        [[ "$status" -eq 0 ]]
    else
        [[ "$status" -ne 0 ]]
    fi
    for stage in "${stages[@]}"; do
        fail_fast=1
        [[ "$stage" != ordinary ]] || fail_fast=0
        printf '%s\t%s\n' "$stage" "$fail_fast"
        [[ "$stage" != "$failure" ]] || break
    done > "$scratch/expected.tsv"
    diff -u "$scratch/expected.tsv" "$scratch/trace.tsv"
    if [[ "$failure" == ordinary ]]; then
        [[ ! -e "$scratch/server.pid" ]]
    else
        read -r server_pid < "$scratch/server.pid"
        if kill -0 "$server_pid" 2>/dev/null; then
            echo "runner left its fixture server running after $failure" >&2
            exit 1
        fi
    fi
done
echo 'workspace test runner failure barriers and cleanup passed'
