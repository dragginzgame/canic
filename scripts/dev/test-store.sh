#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/store
scope=${1:-all}
case "$scope" in
    all) filter= ;;
    db-roles) filter=store_root_tests::db_role_tests ;;
    directory) filter=store_root_tests::directory_tests ;;
    sharding) filter=store_root_tests::sharding_tests ;;
    user-directory) filter=store_root_tests::user_directory_tests ;;
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
    *) printf 'Usage: %s [all|db-roles|directory|user-directory|sharding|readiness|publication|dispatch|custody|admission|quiescence|maintenance|preservation|removal|root-maintenance]\n' "$0" >&2; exit 2 ;;
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
if [[ "$scope" == all ]]; then
    "$cli" app build --config apps/examples/miner-pool/canic.toml
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test miner
fi
if [[ "$scope" == all || "$scope" == db-roles ]]; then
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-schema-role --output target/store/db-role-schema.wasm --json > target/store/db-role-schema.json
    build db-role-root canic-root --features auth,role-issuer
    build db-role-hub ex-user-directory-hub --features role-service
    build db-role-shard ex-user-directory-shard --features role-service
fi
if [[ "$scope" == db-roles ]]; then
    build original ic-wasm-store
    CANIC_STORE_APP_DIRECTORY="$root/target/store" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
if [[ "$scope" == all || "$scope" == directory ]]; then
    cargo build --locked --release -p canic-root --features role-issuer --bin canic-root-client --message-format=json-render-diagnostics > "$records"
    cp -- "$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic-root-client" and .executable != null) | .executable] | if length == 1 then .[0] else error("expected role-enabled Root client") end' "$records")" target/store/role-root.client
    build dormant canic-dormant
    build role-root canic-root --features role-issuer
    build directory canic-directory-witness --manifest qualification/Cargo.toml --features placement,role-probe
fi
if [[ "$scope" == directory ]]; then
    build pool-root canic-root --features sharding
    build root canic-root
    build original ic-wasm-store
    CANIC_STORE_APP_DIRECTORY="$root/target/store" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
if [[ "$scope" == all || "$scope" == user-directory ]]; then
    "$cli" app build --config apps/examples/user-directory/canic.toml
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features inspection --test inspection
fi
if [[ "$scope" == user-directory ]]; then
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
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
    build app canic-greeting-witness --manifest qualification/Cargo.toml
    build app-v2 canic-greeting-witness --manifest qualification/Cargo.toml --features greeting-v2
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
