#!/usr/bin/env bash
set -euo pipefail
measurement_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
app="$measurement_root/app"
results="$measurement_root/results"
canic_bin="$measurement_root/tools/canic"
export CARGO_TARGET_DIR="$measurement_root/target"
export CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
export RUSTC_WRAPPER='' CARGO_TERM_COLOR=never NO_COLOR=1
unset CARGO_BUILD_BUILD_DIR RUSTC_WORKSPACE_WRAPPER
mkdir "$results"
measure() {
    local label="$1" result="$results/$1" status=0
    mkdir "$result"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$result/started.txt"
    (
        cd "$app"
        /usr/bin/time -o "$result/resources.json" -f '{"elapsed_seconds":%e,"user_seconds":%U,"system_seconds":%S,"max_process_rss_kib":%M,"exit_code":%x}' \
            "$canic_bin" --environment staging build toko_miner --profile release --verbose
    ) > "$result/build.log" 2>&1 || status=$?
    date -u +%Y-%m-%dT%H:%M:%SZ > "$result/finished.txt"
    printf '%s\n' "$status" > "$result/exit-code.txt"
    if [[ "$status" -ne 0 ]]; then
        tail -40 "$result/build.log"
        return "$status"
    fi
    (
        cd "$app"
        find .canic/release-builds -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum
    ) > "$result/release-files.sha256"
    cp "$app/Cargo.lock" "$result/Cargo.lock"
    printf '%s ' "$label"
    cat "$result/resources.json"
}
cp "$app/apps/toko_miner/game_shard/src/lib.rs" "$measurement_root/game-shard.rs.original"
cp "$app/crates/toko-miner-contracts/src/lib.rs" "$measurement_root/contracts.rs.original"
restore_inputs() {
    cp "$measurement_root/game-shard.rs.original" "$app/apps/toko_miner/game_shard/src/lib.rs"
    cp "$measurement_root/contracts.rs.original" "$app/crates/toko-miner-contracts/src/lib.rs"
}
trap restore_inputs EXIT
measure cold
measure warm
cmp "$results/cold/release-files.sha256" "$results/warm/release-files.sha256"
printf '\n// Controlled measurement: application compiler input changed.\n' >> "$app/apps/toko_miner/game_shard/src/lib.rs"
measure application_changed
measure application_warm
cmp "$results/application_changed/release-files.sha256" "$results/application_warm/release-files.sha256"
restore_inputs
printf '\n// Controlled measurement: path dependency compiler input changed.\n' >> "$app/crates/toko-miner-contracts/src/lib.rs"
measure dependency_changed
measure dependency_warm
cmp "$results/dependency_changed/release-files.sha256" "$results/dependency_warm/release-files.sha256"
