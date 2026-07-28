#!/usr/bin/env bash

# Shared packaged/installed CLI proof for the current Coordinator catalog
# boundary. Live blob-storage routing remains fenced until it consumes current
# Coordinator/Registry/root topology.

prepare_blob_storage_cli_fixture() {
    local downstream_root="$1"
    local canonical_network_id="402b3681453fb9cfa356b6ea7abde2de53cc4c665caaf438535bddc1e1679f60"
    local fleet_id="0707070707070707070707070707070707070707070707070707070707070707"

    mkdir -p "$downstream_root/.canic/networks/$canonical_network_id/fleets"

    cat > "$downstream_root/.canic/networks/$canonical_network_id/fleets/catalog.json" <<EOF
{
  "schema_version": 1,
  "canonical_network_id": "$canonical_network_id",
  "entries": [{
    "canonical_network_id": "$canonical_network_id",
    "fleet_id": "$fleet_id",
    "fleet_name": "downstream",
    "app": "downstream",
    "environment": "fixture",
    "deployed_at_unix_secs": 1,
    "coordinator_principal": "ryjl3-tyaaa-aaaaa-aaaba-cai"
  }]
}
EOF
}

prepare_fake_blob_storage_icp() {
    local fake_icp="$1"
    local fake_icp_state="$2"

    printf 'unused\n' > "$fake_icp_state"
    cat > "$fake_icp" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

for arg in "$@"; do
    if [ "$arg" = "--version" ]; then
        echo "icp-cli 1.2.0"
        exit 0
    fi
done

echo "unexpected fake icp invocation: $*" >&2
exit 64
EOF
    chmod +x "$fake_icp"
}

run_blob_storage_cli_probe_commands() {
    local runner="$1"
    local proof_root="$2"
    local fake_icp="$3"
    local status_exit=0

    set +e
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage status downstream app --json \
        > "$proof_root/blob-storage-coordinator-fence.out" \
        2> "$proof_root/blob-storage-coordinator-fence.err"
    status_exit=$?
    set -e
    if [ "$status_exit" -ne 1 ]; then
        echo "expected Coordinator-anchored blob-storage status to exit 1, got $status_exit" >&2
        sed -n '1,120p' "$proof_root/blob-storage-coordinator-fence.out" >&2
        sed -n '1,160p' "$proof_root/blob-storage-coordinator-fence.err" >&2
        exit 1
    fi
}

assert_blob_storage_cli_file_contains() {
    local proof_label="$1"
    local description="$2"
    local pattern="$3"
    local path="$4"
    local preview_range="$5"

    grep -Fq -- "$pattern" "$path" || {
        echo "expected $proof_label $description" >&2
        sed -n "$preview_range" "$path" >&2
        exit 1
    }
}

assert_blob_storage_cli_probe_outputs() {
    local proof_label="$1"
    local proof_root="$2"

    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "canic CLI to expose blob-storage help" \
        'Inspect and provision blob-storage billing' \
        "$proof_root/blob-storage-help.out" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage help to list sync-gateways" \
        'sync-gateways' \
        "$proof_root/blob-storage-help.out" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage help to show fund --cycles examples" \
        'canic blob-storage fund local backend --cycles' \
        "$proof_root/blob-storage-help.out" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage help to list check-ready status" \
        '--check-ready' \
        "$proof_root/blob-storage-help.out" \
        '1,180p'

    [ ! -s "$proof_root/blob-storage-status-json.out" ] || {
        echo "expected $proof_label missing-state failure to leave stdout empty" >&2
        sed -n '1,160p' "$proof_root/blob-storage-status-json.out" >&2
        exit 1
    }
    assert_blob_storage_error_contract \
        "$proof_label" \
        "$proof_root/blob-storage-status-json.err"

    [ ! -s "$proof_root/blob-storage-coordinator-fence.out" ] || {
        echo "expected $proof_label Coordinator fence to leave stdout empty" >&2
        sed -n '1,160p' "$proof_root/blob-storage-coordinator-fence.out" >&2
        exit 1
    }
    assert_blob_storage_error_contract \
        "$proof_label" \
        "$proof_root/blob-storage-coordinator-fence.err"
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "Coordinator catalog failure to identify the removed resolver" \
        'Coordinator-anchored' \
        "$proof_root/blob-storage-coordinator-fence.err" \
        '1,160p'
}

assert_blob_storage_error_contract() {
    local proof_label="$1"
    local path="$2"

    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error schema" \
        '"schema_version": 1' \
        "$path" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error kind" \
        '"kind": "blob_storage_error"' \
        "$path" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error target" \
        '"input": "app"' \
        "$path" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error code" \
        '"code": "target_resolution_failed"' \
        "$path" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error exit code" \
        '"exit_code": 1' \
        "$path" \
        '1,160p'
}
