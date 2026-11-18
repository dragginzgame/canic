#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/store
scope=${1:-all}
case "$scope" in
    all) filter= ;;
    sharding) filter=store_root_tests::sharding_tests ;;
    readiness) filter=store_readiness_ ;;
    publication) filter=store_root_tests::publication_tests ;;
    dispatch) filter=store_root_tests::dispatch_tests ;;
    custody) filter=store_custody_observation ;;
    admission) filter=store_root_tests::admission_tests ;;
    quiescence) filter=store_root_tests::quiescence_tests ;;
    maintenance) filter=store_root_tests::maintenance_ ;;
    preservation) filter=store_root_tests::preservation_limits_tests ;;
    removal) filter=store_root_tests::removal_tests ;;
    root-maintenance) filter=store_root_tests::root_custody_tests ;;
    *) printf 'Usage: %s [all|sharding|readiness|publication|dispatch|custody|admission|quiescence|maintenance|preservation|removal|root-maintenance]\n' "$0" >&2; exit 2 ;;
esac
records=$(mktemp target/store/cli.XXXXXX.jsonl)
cargo build --locked -p canic-cli --message-format=json-render-diagnostics > "$records"
cli=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one CLI") end' "$records")
build() {
    local name=$1 package=$2
    shift 2
    # The public builder owns artifact selection and retained build evidence.
    "$cli" artifact build --package "$package" "$@" --json > "target/store/$name.json"
    cp -- "$(jq -r .wasm "target/store/$name.json")" "target/store/$name.wasm"
}
if [[ "$scope" == all || "$scope" == sharding ]]; then
    cargo test --locked -p canic --features sharding sharding::tests
    cargo test --locked -p canic-management --features sharding placement::
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-db-witness --features managed-fixture --output target/store/pool-db.wasm --json > target/store/pool-db.json
    build pool-root canic-root --features sharding
    build pool-faults canic-root --features sharding,fault-injection
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-shard-hub --output target/store/hub.wasm --json > target/store/hub.json
fi
build original ic-wasm-store
build root-maintenance canic-root --features maintenance-candidate
build root-faults canic-root --features fault-injection
if [[ "$scope" == all || "$scope" == root-maintenance ]]; then
    records=$(mktemp target/store/host-tests.XXXXXX.jsonl)
    cargo test --locked -p canic-host --lib --no-run --message-format=json-render-diagnostics > "$records"
    CANIC_HOST_CUSTODY_TEST=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic_host") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected Host test executable") end' "$records")
    export CANIC_HOST_CUSTODY_TEST
fi
if [[ "$scope" == all || "$scope" == sharding || "$scope" == publication || "$scope" == dispatch || "$scope" == root-maintenance ]]; then
    build root canic-root
    build app canic-hello
    build app-v2 canic-hello --features greeting-v2
fi
if [[ "$scope" == all || "$scope" == custody || "$scope" == admission || "$scope" == quiescence || "$scope" == maintenance || "$scope" == preservation || "$scope" == root-maintenance ]]; then
    build candidate ic-wasm-store --features maintenance-candidate
fi
if [[ "$scope" == all || "$scope" == custody || "$scope" == admission ]]; then
    # Supplier-native tests live in ic-wasm-store; this runner owns Canic integration.
    cargo test --locked -p canic-management-protocol --lib
elif [[ "$scope" == root-maintenance ]]; then
    cargo test --locked -p canic-management-protocol --lib complete_root_custody
elif [[ "$scope" == quiescence || "$scope" == maintenance || "$scope" == preservation ]]; then
    cargo test --locked -p canic-management-protocol --lib store_maintenance
elif [[ "$scope" == dispatch ]]; then
    cargo test --locked -p canic-management-protocol --lib admission
    cargo test --locked -p canic-management-protocol --lib upgrade_binds
elif [[ "$scope" != sharding ]]; then
    case "$scope" in readiness) native=store_readiness ;; publication) native=publication_envelope ;; removal) native=removal_envelope ;; esac
    cargo test --locked -p canic-management-protocol --lib "$native"
fi
CANIC_CLI="$cli" CANIC_EFFECT_PROBE_WASM="$root/target/store/original.wasm" \
    CANIC_STORE_APP_DIRECTORY="$root/target/store" \
    cargo test --locked --manifest-path qualification/Cargo.toml \
    -p canic-qualification --test store "$filter" -- --nocapture
