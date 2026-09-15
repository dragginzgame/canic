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
git -C "$ROOT" worktree remove "$SCRATCH/product"

HASH_CHECK_ROOT="$SCRATCH/hash-check"
mkdir -p "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches"
cp "$RUNNER" "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh"
cp "$ROOT/scripts/ci/wasm-ablation-artifacts.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
cp "$ROOT/scripts/ci/wasm-ablation-build-artifact.rs" "$HASH_CHECK_ROOT/scripts/ci/"
cp "$COUNTER_SOURCE" "$HASH_CHECK_ROOT/scripts/ci/wasm-replica-function-count.rs"
cp "$ROOT"/scripts/ci/wasm-ablation-patches/*.patch \
    "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-patches/"
cp "$ROOT/scripts/ci/wasm-ablation-experiments.tsv" "$HASH_CHECK_ROOT/scripts/ci/"
# This method copy deliberately has no product source paths. The frozen Git tree
# supplies those inputs; changes in the active checkout must not rebase an ablation.
bash "$HASH_CHECK_ROOT/scripts/ci/wasm-ablation-report.sh" --check >/dev/null
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
    END { if (!rows) exit 1 }
'
printf '%s\n' "$LISTING" | rg -q $'^01\tb1-01-current-baseline\tready\tnone\t'
printf '%s\n' "$LISTING" | rg -q $'^02\tb1-02-global-storage-registration\tready\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^03\tb1-03-activation-record-codecs\tready\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^04\tb1-04-authorization-record-codecs\tready\tpatch\tcanonical,runtime_probe$'
printf '%s\n' "$LISTING" | rg -q $'^05\tb1-05-relevant-cbor-stub\tready\tpatch\tcanonical,runtime_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^06\tb1-06-unconditional-recovery-dispatch\tready\tpatch\tcanonical,runtime_probe$'
printf '%s\n' "$LISTING" | rg -q $'^07\tb1-07-exact-role-capability-expansion\tplanned\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^08\tb1-08-endpoint-candid-type-construction\tready\tpatch\tcanonical,runtime_probe,payload_limit_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^09\tb1-09-candid-type-documentation\tplanned\tpatch\tcanonical,runtime_probe,payload_limit_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^10\tb1-10-candid-serialization-newtypes\tready\tpatch\tcanonical,runtime_probe,payload_limit_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^11\tb1-11-payload-limited-async-adapters\tready\tpatch\tpayload_limit_probe$'
printf '%s\n' "$LISTING" | rg -q $'^12\tb1-12-metrics-providers\tready\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^17\tb1-17-page-generic-cohort\tready\tenv_matrix\tleaf_probe$'
printf '%s\n' "$LISTING" | rg -q $'^18\tb1-18-pool-ledger-hard-cut\tplanned\tcross_commit\tcanonical$'
if printf '%s\n' "$LISTING" | rg -qi 'toko'; then
    echo "consumer-specific artifact entered the Canic ablation listing" >&2
    exit 1
fi

echo "Wasm ablation report tests passed"
