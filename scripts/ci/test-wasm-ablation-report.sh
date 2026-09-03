#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/ci/wasm-ablation-report.sh"
COUNTER_SOURCE="$ROOT/scripts/ci/wasm-replica-function-count.rs"
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/canic-wasm-function-count-test.XXXXXX")"
trap 'rm -rf "$SCRATCH"' EXIT

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
bash "$RUNNER" --help | rg -q -- '--artifact <artifact-id>'

if bash "$RUNNER" --check --smoke >/dev/null 2>&1; then
    echo "smoke mode was accepted without an experiment run" >&2
    exit 1
fi
if bash "$RUNNER" --list --artifact canonical_app >/dev/null 2>&1; then
    echo "artifact narrowing was accepted outside smoke mode" >&2
    exit 1
fi

LISTING="$(bash "$RUNNER" --list)"
[[ "$(printf '%s\n' "$LISTING" | wc -l)" -eq 19 ]]
printf '%s\n' "$LISTING" | rg -q $'^01\tb1-01-current-baseline\tready\tnone\t'
printf '%s\n' "$LISTING" | rg -q $'^02\tb1-02-global-storage-registration\tready\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^03\tb1-03-activation-record-codecs\tready\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^04\tb1-04-authorization-record-codecs\tspecified\tpatch\tcanonical,runtime_probe$'
printf '%s\n' "$LISTING" | rg -q $'^05\tb1-05-relevant-cbor-stub\tspecified\tpatch\tcanonical,runtime_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^06\tb1-06-unconditional-recovery-dispatch\tspecified\tpatch\tcanonical,runtime_probe$'
printf '%s\n' "$LISTING" | rg -q $'^07\tb1-07-exact-role-capability-expansion\tplanned\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^08\tb1-08-endpoint-candid-type-construction\tspecified\tpatch\tcanonical,runtime_probe,payload_limit_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^09\tb1-09-candid-type-documentation\tplanned\tpatch\tcanonical,runtime_probe,payload_limit_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^10\tb1-10-candid-serialization-newtypes\tspecified\tpatch\tcanonical,runtime_probe,payload_limit_probe,blob_storage_probe$'
printf '%s\n' "$LISTING" | rg -q $'^11\tb1-11-payload-limited-async-adapters\tready\tpatch\tpayload_limit_probe$'
printf '%s\n' "$LISTING" | rg -q $'^12\tb1-12-metrics-providers\tspecified\tpatch\tcanonical$'
printf '%s\n' "$LISTING" | rg -q $'^17\tb1-17-page-generic-cohort\tready\tenv_matrix\tleaf_probe$'
printf '%s\n' "$LISTING" | rg -q $'^18\tb1-18-pool-ledger-hard-cut\tplanned\tcross_commit\tcanonical$'
if printf '%s\n' "$LISTING" | rg -qi 'toko'; then
    echo "consumer-specific artifact entered the Canic ablation listing" >&2
    exit 1
fi

for experiment in \
    b1-04-authorization-record-codecs \
    b1-05-relevant-cbor-stub \
    b1-06-unconditional-recovery-dispatch \
    b1-08-endpoint-candid-type-construction \
    b1-10-candid-serialization-newtypes \
    b1-12-metrics-providers; do
    if bash "$RUNNER" \
        --experiment "$experiment" \
        --source HEAD \
        --product-root /tmp \
        --output-root /tmp >/dev/null 2>&1; then
        echo "specified but unqualified experiment was runnable: $experiment" >&2
        exit 1
    fi
done

echo "Wasm ablation report tests passed"
