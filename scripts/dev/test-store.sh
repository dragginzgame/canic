#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/store
scope=${1:-all}
case "$scope" in
    all) filter= ;;
    miner) filter=store_root_tests::miner_pool ;;
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
    *) printf 'Usage: %s [all|miner|db-roles|directory|user-directory|sharding|readiness|publication|dispatch|custody|admission|quiescence|maintenance|preservation|removal|root-maintenance]\n' "$0" >&2; exit 2 ;;
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
build_hub_example() {
    local name=$1
    cargo build --locked --release --target wasm32-unknown-unknown --manifest-path qualification/Cargo.toml -p canic-shard-hub --example "$name" --message-format=json-render-diagnostics > "$records"
    cp -- "$(jq -sr '[.[] | select(.reason == "compiler-artifact") | .filenames[] | select(endswith(".wasm"))] | last' "$records")" "target/store/$name.wasm"
}
build root canic-root
if [[ "$scope" == all || "$scope" == miner ]]; then
    "$cli" app build --config apps/examples/miner-pool/canic.toml
    "$cli" --json artifact endpoints apps/examples/miner-pool/artifacts/root.wasm |
        jq -e '[.endpoints[].name] | contains(["acquire_placement","lookup_placement","refill_standby"]) and (index("assign_shard") == null) and (index("assign_instance") == null)' >/dev/null
    build_hub_example miner-payer
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test miner
fi
if [[ "$scope" == all || "$scope" == db-roles ]]; then
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-schema-role --output target/store/db-role-schema.wasm --json > target/store/db-role-schema.json
    build db-role-receiver canic-schema-role --manifest qualification/Cargo.toml --features receiver
    build db-role-blob-receiver canic-schema-role --manifest qualification/Cargo.toml --features blob-receiver
    cargo run --quiet --locked --manifest-path qualification/Cargo.toml -p canic-schema-role \
        --features inspection --example canic-schema-role-wire > target/store/db-role-wire.json
    jq -e '.echo_id | .mode == "query" and .request == .reply and
        (.request.rust | endswith("::Ulid")) and
        any(.request.schema[]; .k == "array" and .max == 16)' target/store/db-role-wire.json >/dev/null
    "$cli" --json artifact endpoints target/store/db-role-schema.wasm |
        jq -e 'any(.endpoints[]; .name == "echo_id" and .mode == "query")' >/dev/null
    build_hub_example registry-payer
    build db-role-project canic-directory-witness --manifest qualification/Cargo.toml --features funding
    cp -- target/store/registry-payer.wasm target/store/db-role-hub.wasm
    build hub canic-shard-hub --manifest qualification/Cargo.toml
    build_hub_example attested-shard
    cp -- target/store/attested-shard.wasm target/store/db-role-shard.wasm
    cp -- apps/examples/user-directory/shard-nonce.bin target/store/db-role-nonce.bin
fi
if [[ "$scope" == db-roles ]]; then
    build original ic-wasm-store
    CANIC_STORE_APP_DIRECTORY="$root/target/store" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
if [[ "$scope" == all || "$scope" == directory ]]; then
    cargo build --locked --release -p canic-root --bin canic-root-client --message-format=json-render-diagnostics > "$records"
    cp -- "$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic-root-client" and .executable != null) | .executable] | if length == 1 then .[0] else error("expected Root client") end' "$records")" target/store/root.client
    cargo build --locked -p example-user-directory-hub --features operator-client --bin example-user-directory-hub-client --message-format=json-render-diagnostics > "$records"
    cp -- "$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "example-user-directory-hub-client") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected hub codec") end' "$records")" target/store/hub.client
    build recycled-miner example-miner-pool-miner
    build dormant canic-dormant
    "$cli" --json artifact endpoints "target/store/root.wasm" |
        jq -e '[.endpoints[].name] as $names |
            ($names | contains(["acquire_placement", "approve_child_creation", "configure_subnet_capacity", "increase_pool_limit", "configure_child_funding", "stop_child_funding"])) and
            all("configure_capacity", "increase_pool", "canic_configure_shard_routes", "canic_observe_shard", "canic_begin_shard_drain", "configure_shards", "configure_instances", "configure_workers", "configure_singleton", "stop_application_child_funding"; . as $name | ($names | index($name) == null))' >/dev/null
    help=$(cargo run --quiet --locked -p canic-root --bin canic-root-client -- encode --help)
    for action in configure-child-funding stop-child-funding approve-child-creation configure-subnet-capacity; do
        grep -qw -- "$action" <<<"$help"
    done
    build directory canic-directory-witness --manifest qualification/Cargo.toml --features placement,role-probe,funding
fi
if [[ "$scope" == directory ]]; then
    build original ic-wasm-store
    CANIC_CLI="$cli" CANIC_STORE_APP_DIRECTORY="$root/target/store" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
if [[ "$scope" == all || "$scope" == user-directory ]]; then
    "$cli" app build --config apps/examples/user-directory/canic.toml
    "$cli" --json artifact endpoints apps/examples/user-directory/artifacts/user_hub.wasm |
        jq -e '[.endpoints[].name] | contains(["canic_configure_shard_routes","canic_observe_shard","canic_begin_shard_drain"]) and (index("approve_child_creation") == null)' >/dev/null
    "$cli" --json artifact endpoints apps/examples/user-directory/artifacts/root.wasm |
        jq -e '[.endpoints[].name] | contains(["acquire_placement"]) and (index("assign_shard") == null) and (index("lookup_shard") == null) and (index("release_shard") == null) and (index("assign_worker") == null) and (index("refill_standby") != null)' >/dev/null
fi
if [[ "$scope" == user-directory ]]; then
    cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
build root-faults canic-root --features fault-injection
for variant in root root-faults; do
    "$cli" --json artifact endpoints "target/store/$variant.wasm" > "target/store/$variant.endpoints.json"
done
jq -se 'map(.endpoints) as [$root, $faults] |
    ($root - $faults) == [] and
    (($faults - $root) | map({name, mode}) | sort_by(.name)) ==
        [{name:"inject_child_fault", mode:"update"}, {name:"inject_snapshot_fault", mode:"update"}]' \
    target/store/{root,root-faults}.endpoints.json >/dev/null
if [[ "$scope" == miner ]]; then
    CANIC_CLI="$cli" CANIC_STORE_APP_DIRECTORY="$root/target/store" cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store "$filter" -- --nocapture
    exit
fi
if [[ "$scope" == all || "$scope" == sharding ]]; then
    cargo check --locked -p canic --features receiver-role --test durable_routes --target wasm32-unknown-unknown
    cargo test --locked -p canic --features sharding,storage sharding::
    cargo test --locked -p canic-management --features sharding placement::
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-db-witness --features managed-fixture --output target/store/pool-db.wasm --json > target/store/pool-db.json
    "$cli" artifact build --manifest qualification/Cargo.toml --package canic-shard-hub --output target/store/hub.wasm --json > target/store/hub.json
fi
build original ic-wasm-store
bash scripts/dev/prepare-root-maintenance-fixture.sh target/store/root.wasm target/store/root-maintenance.wasm
if [[ "$scope" == all || "$scope" == root-maintenance ]]; then
    records=$(mktemp target/store/host-tests.XXXXXX.jsonl)
    cargo test --locked -p canic-host --lib --no-run --message-format=json-render-diagnostics > "$records"
    CANIC_HOST_CUSTODY_TEST=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic_host") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected Host test executable") end' "$records")
    export CANIC_HOST_CUSTODY_TEST
fi
if [[ "$scope" == all || "$scope" == sharding || "$scope" == publication || "$scope" == dispatch || "$scope" == root-maintenance ]]; then
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
