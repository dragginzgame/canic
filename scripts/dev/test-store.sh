#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/store
scope=${1:-all}
case "$scope" in
    all) filter= ;;
    readiness) filter=store_root_tests::readiness_tests ;;
    publication) filter=store_root_tests::publication_tests ;;
    *) printf 'Usage: %s [all|readiness|publication]\n' "$0" >&2; exit 2 ;;
esac
build() {
    local name=$1 package=$2
    shift 2
    # The public builder owns artifact selection and retained build evidence.
    canic artifact build --package "$package" "$@" --json > "target/store/$name.json"
    cp -- "$(jq -r .wasm "target/store/$name.json")" "target/store/$name.wasm"
}
build original canic-wasm-store
build root-maintenance canic-root --features maintenance-candidate
build root-faults canic-root --features fault-injection
if [[ "$scope" != readiness ]]; then
    build root canic-root
    build app canic-hello
    build app-v2 canic-hello --features greeting-v2
fi
if [[ "$scope" == all ]]; then
    build candidate canic-wasm-store --features maintenance-candidate
    cargo test --locked -p canic-management-protocol -p canic-wasm-store-protocol -p canic-wasm-store --lib
else
    case "$scope" in readiness) native=store_readiness ;; publication) native=publication_envelope ;; esac
    cargo test --locked -p canic-management-protocol --lib "$native"
fi
CANIC_EFFECT_PROBE_WASM="$root/target/store/original.wasm" \
    CANIC_STORE_CANDIDATE="$root/target/store/candidate.wasm" \
    CANIC_STORE_APP_DIRECTORY="$root/target/store" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification --test store "$filter" -- --nocapture
