#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/ci/wasm-ablation-report.sh"
COUNTER_SOURCE="$ROOT/scripts/ci/wasm-replica-function-count.rs"
mkdir -p "$ROOT/.tmp"
SCRATCH="$(mktemp -d "$ROOT/.tmp/canic-wasm-function-count-test.XXXXXX")"
cleanup() {
    if [[ -f "$SCRATCH/product/.git" ]]; then
        git -C "$ROOT" worktree remove "$SCRATCH/product"
    fi
    if [[ -f "$SCRATCH/historical-product/.git" ]]; then
        git -C "$ROOT" worktree remove "$SCRATCH/historical-product"
    fi
    rm -rf "$SCRATCH"
}
trap cleanup EXIT

rustc --edition 2024 -D warnings -C debuginfo=0 -C strip=symbols \
    --remap-path-prefix "$ROOT=canic" \
    "$COUNTER_SOURCE" -o "$SCRATCH/wasm-replica-function-count"
printf '\x00\x61\x73\x6d\x01\x00\x00\x00' >"$SCRATCH/empty.wasm"
printf '\x00\x61\x73\x6d\x01\x00\x00\x00\x01\x04\x01\x60\x00\x00\x03\x02\x01\x00\x0a\x04\x01\x02\x00\x0b' >"$SCRATCH/one-local.wasm"
printf '\x00\x61\x73\x6d\x01\x00\x00\x00\x01\x04\x01\x60\x00\x00\x02\x09\x01\x03\x65\x6e\x76\x01\x66\x00\x00\x03\x02\x01\x00\x0a\x04\x01\x02\x00\x0b' >"$SCRATCH/import-and-local.wasm"
printf '\x00\x61\x73\x6d\x01\x00\x00\x00\x01\x04\x01\x60\x00\x00\x03\x02\x01\x00\x0a\x01\x00' >"$SCRATCH/mismatched.wasm"

wasm-validate "$SCRATCH/empty.wasm"
wasm-validate "$SCRATCH/one-local.wasm"
wasm-validate "$SCRATCH/import-and-local.wasm"
[[ "$("$SCRATCH/wasm-replica-function-count" "$SCRATCH/empty.wasm")" == "0" ]]
[[ "$("$SCRATCH/wasm-replica-function-count" "$SCRATCH/one-local.wasm")" == "1" ]]
[[ "$("$SCRATCH/wasm-replica-function-count" "$SCRATCH/import-and-local.wasm")" == "1" ]]
"$SCRATCH/wasm-replica-function-count" --identity |
    rg -q '^canic-b1-replica-function-count/v1\tic_source_commit=2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a\tquantity=local-defined-functions\tlimit=50000$'
if "$SCRATCH/wasm-replica-function-count" "$SCRATCH/mismatched.wasm" >/dev/null 2>&1; then
    echo "mismatched function and code counts were accepted" >&2
    exit 1
fi

bash "$RUNNER" --check >/dev/null
bash "$RUNNER" --help | rg -q -- '--smoke'
bash "$RUNNER" --help | rg -q -- '--qualify'
bash "$RUNNER" --help | rg -q -- '--artifact <artifact-id>'

# Starting in a later method checkout must not select its Rust toolchain for
# the frozen product's harness preparation or provenance. Stop at the first
# Cargo invocation so this regression does not compile a product artifact.
git -C "$ROOT" worktree add --quiet --detach "$SCRATCH/product" \
    50f40171d6177c3d1e490b1fdb5f6163323b2cd5
mkdir -p "$SCRATCH/bin"
cat >"$SCRATCH/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
pwd >"$B1_TEST_CARGO_CWD"
printf '%s\n' "$@" >"$B1_TEST_CARGO_ARGS"
if [[ "${B1_TEST_DRIVER:-0}" == 1 ]]; then
    printf '%s\n' "$1" >>"$B1_TEST_DRIVER_STEPS"
    if [[ "$1" == metadata ]]; then
        exit 0
    fi
    [[ "$1" == build ]]
    printf '%s\n' "$CARGO_TARGET_DIR" >"$B1_TEST_NATIVE_TARGET"
    mkdir -p "$CARGO_TARGET_DIR/fast"
    cat >"$CARGO_TARGET_DIR/fast/canic-wasm-ablation-build-artifact" <<'DRIVER'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$CARGO_TARGET_DIR" >"$B1_TEST_ARTIFACT_TARGET"
printf '%s\n' "$@" >"$B1_TEST_DRIVER_ARGS"
if [[ "${B1_TEST_ARTIFACT_DRIFT:-0}" == 1 ]]; then
    printf '\n' >>Cargo.lock
    artifact_dir="$PWD/.icp/local/canisters/$1"
    mkdir -p "$artifact_dir"
    printf '\x00\x61\x73\x6d\x01\x00\x00\x00' >"$artifact_dir/$1.wasm"
    gzip -nc "$artifact_dir/$1.wasm" >"$artifact_dir/$1.wasm.gz"
    printf 'service : {}\n' >"$artifact_dir/$1.did"
    exit 0
fi
exit 92
DRIVER
    chmod +x "$CARGO_TARGET_DIR/fast/canic-wasm-ablation-build-artifact"
    exit 0
fi
if [[ "${B1_TEST_PREPARED_DOCS:-0}" == 1 ]]; then
    [[ -f .cargo/config.toml ]]
    [[ -f crates/canic-macros/audit/candid_derive/src/derive.rs ]]
fi
if [[ "${B1_TEST_HISTORICAL:-0}" == 1 ]]; then
    [[ -f apps/test/index_child/src/lib.rs ]]
    [[ -f apps/test/index_hub/src/lib.rs ]]
    [[ -f crates/canic-host/src/bootstrap_pool_ledger_recovery.rs ]]
fi
if [[ "${B1_TEST_MUTATE_PREPARED_DOCS:-0}" == 1 ]]; then
    source_path=crates/canic-macros/audit/candid_derive/src/derive.rs
    cp "$source_path" "$B1_TEST_SOURCE_BACKUP"
    printf '\n// injected source drift\n' >>"$source_path"
    exit 0
fi
exit 91
EOF
chmod +x "$SCRATCH/bin/cargo"
if (
    cd "$ROOT"
    PATH="$SCRATCH/bin:$PATH" TMPDIR="$SCRATCH" \
        B1_TEST_CARGO_CWD="$SCRATCH/cargo-cwd" \
        B1_TEST_CARGO_ARGS="$SCRATCH/cargo-args" \
        bash "$RUNNER" --smoke --experiment b1-06-unconditional-recovery-dispatch \
            --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
            --product-root "$SCRATCH/product" --output-root "$SCRATCH/output"
) >"$SCRATCH/toolchain-context.log" 2>&1; then
    echo "ablation runner ignored the failed Cargo preflight" >&2
    exit 1
fi
[[ "$(cat "$SCRATCH/cargo-cwd")" == "$SCRATCH/product" ]]
[[ "$(head -n 1 "$SCRATCH/cargo-args")" == "metadata" ]]
[[ -z "$(git -C "$SCRATCH/product" status --porcelain=v1)" ]]

# Compile the tool in its own target and execute it with the measured target;
# an artifact failure must still reverse the causal patch and stop the run.
status=0
PATH="$SCRATCH/bin:$PATH" TMPDIR="$SCRATCH" B1_TEST_DRIVER=1 \
    B1_TEST_CARGO_CWD="$SCRATCH/driver-cargo-cwd" \
    B1_TEST_CARGO_ARGS="$SCRATCH/driver-cargo-args" \
    B1_TEST_DRIVER_STEPS="$SCRATCH/driver-steps" \
    B1_TEST_NATIVE_TARGET="$SCRATCH/native-target" \
    B1_TEST_ARTIFACT_TARGET="$SCRATCH/artifact-target" \
    B1_TEST_DRIVER_ARGS="$SCRATCH/driver-args" \
    bash "$RUNNER" --smoke --experiment b1-15-timer-watchdog-providers \
        --artifact canonical_app \
        --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
        --product-root "$SCRATCH/product" --output-root "$SCRATCH/driver-output" \
        >"$SCRATCH/driver.log" 2>&1 || status=$?
[[ "$status" -eq 2 ]]
[[ "$(cat "$SCRATCH/driver-steps")" == $'metadata\nbuild' ]]
[[ "$(cat "$SCRATCH/native-target")" == */harness-target ]]
[[ "$(cat "$SCRATCH/artifact-target")" == */cargo-target ]]
[[ "$(cat "$SCRATCH/native-target")" != "$(cat "$SCRATCH/artifact-target")" ]]
[[ "$(head -n 1 "$SCRATCH/driver-args")" == app ]]
[[ "$(sed -n '2p' "$SCRATCH/driver-args")" == release ]]
[[ -z "$(git -C "$SCRATCH/product" status --porcelain=v1 --untracked-files=all)" ]]

# Reject a successful builder that changes source before accepting its payload
# or scheduling the next artifact. The validator marker detects capture entry.
cat >"$SCRATCH/bin/wasm-validate" <<'EOF'
#!/usr/bin/env bash
touch "$B1_TEST_CAPTURE_ENTERED"
exit 94
EOF
chmod +x "$SCRATCH/bin/wasm-validate"
cp "$SCRATCH/product/Cargo.lock" "$SCRATCH/source-lock"
status=0
PATH="$SCRATCH/bin:$PATH" TMPDIR="$SCRATCH" B1_TEST_DRIVER=1 \
    B1_TEST_ARTIFACT_DRIFT=1 \
    B1_TEST_CARGO_CWD="$SCRATCH/drift-cargo-cwd" \
    B1_TEST_CARGO_ARGS="$SCRATCH/drift-cargo-args" \
    B1_TEST_DRIVER_STEPS="$SCRATCH/drift-driver-steps" \
    B1_TEST_NATIVE_TARGET="$SCRATCH/drift-native-target" \
    B1_TEST_ARTIFACT_TARGET="$SCRATCH/drift-artifact-target" \
    B1_TEST_DRIVER_ARGS="$SCRATCH/drift-driver-args" \
    B1_TEST_CAPTURE_ENTERED="$SCRATCH/capture-entered" \
    bash "$RUNNER" --smoke --experiment b1-15-timer-watchdog-providers \
        --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
        --product-root "$SCRATCH/product" --output-root "$SCRATCH/drift-output" \
        >"$SCRATCH/artifact-drift.log" 2>&1 || status=$?
[[ "$status" -eq 2 && ! -e "$SCRATCH/capture-entered" ]]
[[ "$(head -n 1 "$SCRATCH/drift-driver-args")" == app ]]
cp "$SCRATCH/source-lock" "$SCRATCH/product/Cargo.lock"
[[ -z "$(git -C "$SCRATCH/product" status --porcelain=v1 --untracked-files=all)" ]]
rm "$SCRATCH/bin/wasm-validate"

HASH_CHECK_ROOT="$SCRATCH/hash-check"
mkdir -p "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches"
cp "$RUNNER" "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh"
cp "$ROOT/scripts/ci/wasm-ablation-artifacts.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
cp "$ROOT/scripts/ci/wasm-ablation-build-artifact.rs" "$HASH_CHECK_ROOT/scripts/ci/"
cp "$COUNTER_SOURCE" "$HASH_CHECK_ROOT/scripts/ci/wasm-replica-function-count.rs"
cp "$ROOT"/scripts/ci/wasm-ablation-patches/*.patch \
    "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches/"
cp "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
# Qualification state is a property of this test copy, not catalog progress.
awk -F '\t' 'BEGIN { OFS=FS } $2 == "b1-09-candid-type-documentation" { $3 = "specified" } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
# Common dependency preparation must precede harness metadata and restore on
# failure. Source drift in a newly added audit file must stop before a build.
for drift in 0 1; do
    status=0
    PATH="$SCRATCH/bin:$PATH" TMPDIR="$SCRATCH" \
        B1_TEST_CARGO_CWD="$SCRATCH/prepared-cargo-cwd" \
        B1_TEST_CARGO_ARGS="$SCRATCH/prepared-cargo-args" \
        B1_TEST_PREPARED_DOCS=1 B1_TEST_MUTATE_PREPARED_DOCS="$drift" \
        B1_TEST_SOURCE_BACKUP="$SCRATCH/prepared-source-backup" \
        bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --qualify --experiment b1-09-candid-type-documentation \
            --artifact canonical_app \
            --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
            --product-root "$SCRATCH/product" \
            --output-root "$SCRATCH/prepared-output-$drift" \
            >"$SCRATCH/prepared-$drift.log" 2>&1 || status=$?
    if [[ "$drift" == 0 ]]; then
        [[ "$status" -eq 91 ]]
    else
        [[ "$status" -eq 2 ]]
        # Restore only the test's deliberate edit before reversing preparation.
        cp "$SCRATCH/prepared-source-backup" \
            "$SCRATCH/product/crates/canic-macros/audit/candid_derive/src/derive.rs"
        git -C "$SCRATCH/product" apply --reverse \
            "$ROOT/scripts/ci/wasm-ablation-patches/b1-09-candid-type-documentation.patch"
    fi
    [[ "$(head -n 1 "$SCRATCH/prepared-cargo-args")" == metadata ]]
    [[ "$(cat "$SCRATCH/prepared-cargo-cwd")" == "$SCRATCH/product" ]]
    [[ -z "$(git -C "$SCRATCH/product" status --porcelain=v1 --untracked-files=all)" ]]
done

cp "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
# The historical pair resolves the prepared roster on its own exact anchor,
# keeps the family present in control and restores preparation on failure.
awk -F '\t' 'BEGIN { OFS=FS } $2 == "b1-18-pool-ledger-hard-cut" { $3 = "specified" } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
git -C "$ROOT" worktree add --quiet --detach "$SCRATCH/historical-product" \
    f9009d5ae7be78d4f9dd746431584368770e8364
status=0
PATH="$SCRATCH/bin:$PATH" TMPDIR="$SCRATCH" B1_TEST_HISTORICAL=1 \
    B1_TEST_CARGO_CWD="$SCRATCH/historical-cargo-cwd" \
    B1_TEST_CARGO_ARGS="$SCRATCH/historical-cargo-args" \
    bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --qualify \
        --experiment b1-18-pool-ledger-hard-cut --artifact canonical_index_hub \
        --source f9009d5ae7be78d4f9dd746431584368770e8364 \
        --product-root "$SCRATCH/historical-product" \
        --output-root "$SCRATCH/historical-output" \
        >"$SCRATCH/historical.log" 2>&1 || status=$?
[[ "$status" -eq 91 ]]
[[ "$(head -n 1 "$SCRATCH/historical-cargo-args")" == metadata ]]
[[ "$(cat "$SCRATCH/historical-cargo-cwd")" == "$SCRATCH/historical-product" ]]
[[ -z "$(git -C "$SCRATCH/historical-product" status --porcelain=v1 --untracked-files=all)" ]]
rm "$SCRATCH/historical-cargo-args"
status=0
PATH="$SCRATCH/bin:$PATH" TMPDIR="$SCRATCH" \
    B1_TEST_CARGO_ARGS="$SCRATCH/historical-cargo-args" \
    bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --qualify \
        --experiment b1-18-pool-ledger-hard-cut \
        --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
        --product-root "$SCRATCH/product" --output-root "$SCRATCH/wrong-anchor" \
        >"$SCRATCH/wrong-anchor.log" 2>&1 || status=$?
[[ "$status" -eq 2 && ! -e "$SCRATCH/historical-cargo-args" && ! -e "$SCRATCH/wrong-anchor" ]]
printf '\n' >>"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches/b1-18-historical-roster-preparation.patch"
if bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null 2>&1; then
    echo "historical pair accepted changed preparation" >&2
    exit 1
fi
cp "$ROOT/scripts/ci/wasm-ablation-patches/b1-18-historical-roster-preparation.patch" \
    "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches/"
cp "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
# This method copy deliberately has no product source paths. The frozen Git tree
# supplies those inputs; changes in the active checkout must not rebase an ablation.
bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null
# Historical-only artifacts cannot enter an ordinary frozen-source experiment.
awk -F '\t' 'BEGIN { OFS=FS } $2 == "b1-01-current-baseline" { $7 = $7 ",historical_pool_ledger_recovery" } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
if bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null 2>&1; then
    echo "ordinary experiment accepted a historical-only artifact" >&2
    exit 1
fi
cp "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
# Membership is validated by required role identity rather than aggregate count.
awk -F '\t' '$1 != "canonical_root"' "$ROOT/scripts/ci/wasm-ablation-artifacts.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-artifacts.tsv"
if bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null 2>&1; then
    echo "ablation manifest accepted a missing canonical Root" >&2
    exit 1
fi
cp "$ROOT/scripts/ci/wasm-ablation-artifacts.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
# An accepted source finding must never masquerade as a runnable zero-cost pair.
for source_mode in retained smoke qualify; do
    source_args=()
    if [[ "$source_mode" != retained ]]; then
        source_args=("--$source_mode")
    fi
    status=0
    PATH="$SCRATCH/bin:$PATH" B1_TEST_CARGO_ARGS="$SCRATCH/source-finding-cargo" \
        bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" "${source_args[@]}" \
            --experiment b1-07-exact-role-capability-expansion \
            --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
            --product-root "$SCRATCH/product" --output-root "$SCRATCH/source-finding-$source_mode" \
            >"$SCRATCH/source-finding-$source_mode.log" 2>&1 || status=$?
    [[ "$status" -eq 2 && ! -e "$SCRATCH/source-finding-cargo" && ! -e "$SCRATCH/source-finding-$source_mode" ]]
done
# A prepared matrix must bind its common fixture before any artifact work.
awk -F '\t' 'BEGIN { OFS=FS } $4 == "env_matrix" { $6 = sprintf("%064d", 0) } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
if bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null 2>&1; then
    echo "prepared matrix accepted a mismatched fixture SHA-256" >&2
    exit 1
fi
# A planned matrix must stop before Cargo or output creation in every run mode.
# Set state in the test copy so normal catalog promotion needs no test rewrite.
awk -F '\t' 'BEGIN { OFS=FS } $4 == "env_matrix" { $3 = "planned"; $6 = "-" } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
for run_mode in retained smoke qualify; do
    mode_args=()
    if [[ "$run_mode" != "retained" ]]; then
        mode_args=("--$run_mode")
    fi
    rm -f "$SCRATCH/cargo-cwd" "$SCRATCH/cargo-args"
    status=0
    PATH="$SCRATCH/bin:$PATH" \
        B1_TEST_CARGO_CWD="$SCRATCH/cargo-cwd" \
        B1_TEST_CARGO_ARGS="$SCRATCH/cargo-args" \
        bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" "${mode_args[@]}" \
            --experiment b1-17-page-generic-cohort \
            --source 50f40171d6177c3d1e490b1fdb5f6163323b2cd5 \
            --product-root "$SCRATCH/product" \
            --output-root "$SCRATCH/planned-$run_mode" \
            >"$SCRATCH/planned-$run_mode.log" 2>&1 || status=$?
    [[ "$status" -eq 2 ]]
    [[ ! -e "$SCRATCH/cargo-cwd" && ! -e "$SCRATCH/cargo-args" ]]
    [[ ! -e "$SCRATCH/planned-$run_mode" ]]
    [[ -z "$(git -C "$SCRATCH/product" status --porcelain=v1)" ]]
done
git -C "$ROOT" worktree remove "$SCRATCH/product"

awk -F '\t' 'BEGIN { OFS=FS } $1 == "02" { $6 = sprintf("%064d", 0) } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
if bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null 2>&1; then
    echo "ablation manifest accepted a mismatched patch SHA-256" >&2
    exit 1
fi

# A self-consistent patch digest cannot authorize a patch for another source tree.
cat >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches/b1-02-global-storage-registration.patch" <<'EOF'
diff --git a/missing-ablation-source.rs b/missing-ablation-source.rs
--- a/missing-ablation-source.rs
+++ b/missing-ablation-source.rs
@@ -1 +1 @@
-missing
+changed
EOF
PATCH_SHA256="$(sha256sum "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches/b1-02-global-storage-registration.patch" | awk '{print $1}')"
awk -F '\t' -v digest="$PATCH_SHA256" 'BEGIN { OFS=FS } $1 == "02" { $6 = digest } { print }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" \
    >"$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
if bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null 2>&1; then
    echo "ablation manifest accepted a hash-matched patch for the wrong source" >&2
    exit 1
fi

if bash "$RUNNER" --check --smoke >/dev/null 2>&1; then
    echo "smoke mode was accepted without an experiment run" >&2
    exit 1
fi
if bash "$RUNNER" --check --qualify >/dev/null 2>&1; then
    echo "qualification mode was accepted without an experiment run" >&2
    exit 1
fi
if bash "$RUNNER" --list --smoke --qualify >/dev/null 2>&1; then
    echo "multiple development run modes were accepted" >&2
    exit 1
fi
if bash "$RUNNER" --list --artifact canonical_app >/dev/null 2>&1; then
    echo "artifact narrowing was accepted outside smoke mode" >&2
    exit 1
fi

LISTING="$(bash "$RUNNER" --list)"
printf '%s\n' "$LISTING" | awk -F '\t' '
    NR > 1 { if (sequences[$1]++ || experiments[$2]++) exit 1; rows++ }
    END {
        if (!rows || !experiments["b1-01-current-baseline"] ||
            !experiments["b1-17-page-generic-cohort"]) exit 1
    }
'
# The list command projects structured catalog fields; readiness changes are data.
printf '%s\n' "$LISTING" >"$SCRATCH/listing.tsv"
awk -F '\t' 'BEGIN { OFS=FS } { print $1,$2,$3,$4,$7 }' \
    "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" >"$SCRATCH/expected-listing.tsv"
cmp "$SCRATCH/expected-listing.tsv" "$SCRATCH/listing.tsv"
if printf '%s\n' "$LISTING" | rg -qi 'toko'; then
    echo "consumer-specific artifact entered the Canic ablation listing" >&2
    exit 1
fi

echo "Wasm ablation report tests passed"
