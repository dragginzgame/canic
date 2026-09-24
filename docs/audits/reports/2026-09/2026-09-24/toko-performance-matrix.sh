#!/usr/bin/env bash
set -euo pipefail

matrix_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
original="$matrix_root/original"
relocated="$matrix_root/relocated"
canic_bin="$matrix_root/tools/canic"
results="$matrix_root/results"
export RUSTC_WRAPPER=''
export CARGO_NET_OFFLINE=true
export CARGO_BUILD_JOBS=4
export CARGO_INCREMENTAL=0
export CARGO_TERM_COLOR=never
export NO_COLOR=1
unset CARGO_BUILD_BUILD_DIR RUSTC_WORKSPACE_WRAPPER

# No external commands, environment values or arbitrary process arguments are retained.
process_snapshot() {
    local path name cwd
    for path in /proc/[0-9]*; do
        read -r name 2>/dev/null < "$path/comm" || continue
        case "$name" in cargo|rustc|make|gmake|sccache|pocket-ic)
            cwd="$(readlink "$path/cwd" 2>/dev/null || true)"
            printf '%s\t%s\t%s\n' "${path##*/}" "$name" "$cwd"
            ;;
        esac
    done
}

measure() {
    local label="$1" workspace="$2" case_root="$results/$1" status=0 sampler
    mkdir "$case_root"
    process_snapshot > "$case_root/processes-before.tsv"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$case_root/started.txt"
    printf 'Measuring %s\n' "$label"
    export CARGO_TARGET_DIR="$workspace/target"
    (
        while true; do
            date -u +%Y-%m-%dT%H:%M:%SZ
            process_snapshot
            sleep 30
        done
    ) > "$case_root/processes-during.tsv" &
    sampler=$!
    (
        cd "$workspace"
        /usr/bin/time -o "$case_root/resources.json" -f '{"elapsed_seconds":%e,"user_seconds":%U,"system_seconds":%S,"max_process_rss_kib":%M,"major_faults":%F,"minor_faults":%R,"voluntary_switches":%w,"involuntary_switches":%c,"filesystem_inputs":%I,"filesystem_outputs":%O,"exit_code":%x}' \
            "$canic_bin" --environment staging build toko_miner --profile release
    ) > "$case_root/build.log" 2>&1 || status=$?
    # Only the private sampling helper created immediately above is stopped.
    kill "$sampler" 2>/dev/null || true
    wait "$sampler" 2>/dev/null || true
    date -u +%Y-%m-%dT%H:%M:%SZ > "$case_root/finished.txt"
    process_snapshot > "$case_root/processes-after.tsv"
    printf '%s\n' "$status" > "$case_root/exit-code.txt"
    if [[ "$status" -ne 0 ]]; then
        tail -50 "$case_root/build.log"
        return "$status"
    fi
    if [[ -d "$workspace/.canic/build-reuse" ]]; then
        mkdir "$case_root/reuse"
        cp "$workspace"/.canic/build-reuse/*.json "$case_root/reuse/"
    fi
    (
        cd "$workspace"
        find .canic/release-builds -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum
    ) > "$case_root/release-files.sha256"
    sha256sum "$workspace/Cargo.lock" > "$case_root/cargo-lock.sha256"
    cat "$case_root/resources.json"
}

measure original_cold "$original"
measure original_warm "$original"
# Retain the baseline's exact inputs and evidence for relocation, without mutable Cargo targets.
mkdir "$relocated"
tar -xf "$matrix_root/source.tar" -C "$relocated"
cp -a "$original/.canic" "$relocated/.canic"

note="$original/apps/toko_miner/game_shard/qualification-build-note.md"
printf 'Measurement-only qualification note; no runtime source changes.\n' > "$note"
measure qualification_only "$original"
measure qualification_warm "$original"
rm -- "$note"

source_path=apps/toko_miner/game_shard/src/design/energy_bar.rs
sed -i 's/pub const PRICE: u64 = 5;/pub const PRICE: u64 = 6;/' "$original/$source_path"
measure gameplay_change "$original"
measure gameplay_warm "$original"
tar -xOf "$matrix_root/source.tar" "$source_path" > "$original/$source_path"

source_path=crates/toko-miner-contracts/src/user_sharding.rs
sed -i 's/pub const MAX_ENROLLMENT_OPERATION_ID_LEN: usize = 64;/pub const MAX_ENROLLMENT_OPERATION_ID_LEN: usize = 65;/' "$original/$source_path"
measure dependency_change "$original"
measure dependency_warm "$original"
tar -xOf "$matrix_root/source.tar" "$source_path" > "$original/$source_path"

measure relocated_cold "$relocated"
measure relocated_warm "$relocated"
