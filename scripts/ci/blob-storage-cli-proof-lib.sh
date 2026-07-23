#!/usr/bin/env bash

prepare_blob_storage_cli_fixture() {
    local downstream_root="$1"

    mkdir -p \
        "$downstream_root/.canic/fixture/deployments" \
        "$downstream_root/.icp/fixture/canisters/app" \
        "$downstream_root/.icp/fixture/canisters/root"

    cat > "$downstream_root/.canic/fixture/deployments/downstream.json" <<'EOF'
{
  "schema_version": 1,
  "deployment_name": "downstream",
  "fleet_template": "downstream",
  "created_at_unix_secs": 1,
  "updated_at_unix_secs": 1,
  "environment": "fixture",
  "root_target": "root",
  "root_canister_id": "ryjl3-tyaaa-aaaaa-aaaba-cai",
  "root_verification": "not_verified",
  "root_build_target": "root",
  "workspace_root": ".",
  "icp_root": ".",
  "config_path": "apps/downstream/canic.toml",
  "release_set_manifest_path": ".icp/fixture/canisters/root/release-set.json"
}
EOF

    cat > "$downstream_root/.icp/fixture/canisters/root/root.did" <<'EOF'
service : {
  canic_subnet_registry : () -> () query;
}
EOF

    cat > "$downstream_root/.icp/fixture/canisters/app/app.did" <<'EOF'
service : {
  get_blob_storage_status : (record { sync_gateway_principals : bool }) -> () query;
  "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
  "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
}
EOF
}

prepare_fake_blob_storage_icp() {
    local fake_icp="$1"
    local fake_icp_state="$2"

    printf 'initial\n' > "$fake_icp_state"
    cat > "$fake_icp" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

STATE_FILE="${FAKE_ICP_STATE:?missing FAKE_ICP_STATE}"

emit_response_bytes() {
    printf '{"response_bytes":"%s","response_text":null,"response_candid":"fixture"}\n' "$1"
}

# Exact current Candid responses for the maintained fixture DTOs. Both CLI
# proofs decode these bytes through the same ICP 1.1 boundary as real calls.
for arg in "$@"; do
    if [ "$arg" = "--version" ]; then
        echo "icp-cli 1.1.0"
        exit 0
    fi
done

for arg in "$@"; do
    case "$arg" in
        canic_ready)
            echo 'true'
            exit 0
            ;;
        canic_subnet_registry)
            emit_response_bytes '4449444c096b02bc8a0101c5fed201076d026c03cbb0d50268f6d6bbdd047191edb1ef0f036c05cbb0d50268f6d6bbdd0471aaacd9d0067881cfaef40a04f6fee59a0f066e056d7b6e686c02ade2928e0408c7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f01000002010a0000000000000002010104726f6f74010a0000000000000002010104726f6f7401000000000000000000010a0000000000000001010103617070010a000000000000000101010361707001000000000000000001010a00000000000000020101'
            exit 0
            ;;
        canic_root_issuer_renewal_status)
            emit_response_bytes '4449444c136b02bc8a0101c5fed201116c0382c7abf8010291ecada00808bad09fe20a0b6e036c09c9d4c21204b2ceef2f05cae0e9b701068eecb7aa0478cfc6f7d8047880b5a6f00404bff9f9860a07cca8c8ab0c07c6c2b7be0e786d7b6b06a5d1e6717fc7bdc88f047fe5b6bdc4047f9d82ce9d087ffccbeac80e7f9a9487dd0e7f6e716e786e096c079bdaa996020adaded9f305078dc6e9f90578c5a0c0c6096886adbeda09789f92b59c0c0498ead9fb0d076e046e0c6c05f081a8020dd7b68797010eb3ec93fe06788189c4f1077ec5a0c0c609686b03c7bc99b40768c7c4879a0968b9f5f0990c716d0f6c02dfe0a7ab0410d1e6b3b708716d716c02ade2928e0412c7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f0100000001012003030303030303030303030303030303030303030303030303030303030303030100a8ccb78c907c1600c0556f75907c16010a0000000000000001010100a8ccb78c907c162001010101010101010101010101010101010101010101010101010101010101010100904300a4907c16010204746573740000b864d94500000001010a00000000000000010101'
            exit 0
            ;;
        canic_active_delegation_proof_status)
            emit_response_bytes '4449444c096b02bc8a0101c5fed201076c06c9d4c21202b2ceef2f04eca7c9bf0605c5a0c0c60906c6c2b7be0e05ee88f9db0e066e036d7b6b04c6ced7067fd084d0df0a7fdcc997a70c7f858fee950f7f6e786e686c02ade2928e0408c7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f01000001200404040404040404040404040404040404040404040404040404040404040404020100a8ccb78c907c1601010a000000000000000101010100904300a4907c1601010a00000000000000020101'
            exit 0
            ;;
        get_blob_storage_status)
            state="$(cat "$STATE_FILE")"
            if [ "$state" = "funded" ]; then
                emit_response_bytes '4449444c106b02bc8a0101c5fed2010e6c0fb4ffc21902b9c0b4cd0105ebe9aba80207b0a7ffa50307f781a3c50508e093e98c070797edccc4080982a9e0c7090bced486fb090cc3b78fad0b78bde8c7c00b7db4eed4830d08f0bce79b0e0d8cc4939d0e07e3c0eab50e7e6b069caac29f0403b4b9f0cc047fbedda2e20704889793a4087f91a5fcf10a7f9ff5b4be0f7f6c01fecdfe920c7d6c02fecdfe920c7d87b6f1d20e7d6d066b0697c393a8047fdd9edcc2057fbedda2e2077f91a5fcf10a7fc2e187a00f7f9cd5bfd70f7f6e7d6e686d0a6b0497c393a8047feca2aaa6087fc2e187a00f7fd587e1ec0f7f6e786b03e4808fb6037f94c6cdb4067ffb83b09a0c7f6b02b297c6e1067f91a5fcf10a7f6c02ade2928e040fc7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f010000030001f40301d00f01010a0000000000000002010101ec0e00017b00000000000000000100000000000000b41001010a000000000000000101010001e80701'
            elif [ "$state" = "synced" ]; then
                emit_response_bytes '4449444c106b02bc8a0101c5fed2010e6c0fb4ffc21902b9c0b4cd0105ebe9aba80207b0a7ffa50307f781a3c50508e093e98c070797edccc4080982a9e0c7090bced486fb090cc3b78fad0b78bde8c7c00b7db4eed4830d08f0bce79b0e0d8cc4939d0e07e3c0eab50e7e6b069caac29f0403b4b9f0cc047fbedda2e20704889793a4087f91a5fcf10a7f9ff5b4be0f7f6c01fecdfe920c7d6c02fecdfe920c7d87b6f1d20e7d6d066b0697c393a8047fdd9edcc2057fbedda2e2077f91a5fcf10a7fc2e187a00f7f9cd5bfd70f7f6e7d6e686d0a6b0497c393a8047feca2aaa6087fc2e187a00f7fd587e1ec0f7f6e786b03e4808fb6037f94c6cdb4067ffb83b09a0c7f6b02b297c6e1067f91a5fcf10a7f6c02ade2928e040fc7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f010000008407010501f40301d00f01010a00000000000000020101016400017b00000000000000000100000000000000b81701010a000000000000000101010001e80700'
            else
                emit_response_bytes '4449444c106b02bc8a0101c5fed2010e6c0fb4ffc21902b9c0b4cd0105ebe9aba80207b0a7ffa50307f781a3c50508e093e98c070797edccc4080982a9e0c7090bced486fb090cc3b78fad0b78bde8c7c00b7db4eed4830d08f0bce79b0e0d8cc4939d0e07e3c0eab50e7e6b069caac29f0403b4b9f0cc047fbedda2e20704889793a4087f91a5fcf10a7f9ff5b4be0f7f6c01fecdfe920c7d6c02fecdfe920c7d87b6f1d20e7d6d066b0697c393a8047fdd9edcc2057fbedda2e2077f91a5fcf10a7fc2e187a00f7f9cd5bfd70f7f6e7d6e686d0a6b0497c393a8047feca2aaa6087fc2e187a00f7fd587e1ec0f7f6e786b03e4808fb6037f94c6cdb4067ffb83b09a0c7f6b02b297c6e1067f91a5fcf10a7f6c02ade2928e040fc7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f01000000840702010501f40301d00f01010a0000000000000002010101640000000000000000000000b81701010a000000000000000101010001e80700'
            fi
            exit 0
            ;;
        _immutableObjectStorageUpdateGatewayPrincipals)
            printf 'synced\n' > "$STATE_FILE"
            emit_response_bytes '4449444c036b02bc8a017fc5fed201016c02ade2928e0402c7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f010000'
            exit 0
            ;;
        _immutableObjectStorageFundFromProjectCycles)
            printf 'funded\n' > "$STATE_FILE"
            emit_response_bytes '4449444c056b02bc8a0101c5fed201036c07b3bffd8501028bd3b5cf087d90edb69e097dc89791a70a7de5959eac0b7dfecdfe920c7df0cf81aa0f7d6e716c02ade2928e0404c7ebc4d009716b19ddc6a0117ff2a2c990017fa0bff4cd017f87a2e58c027fb38b96dc027fe5fc9ff6027f96d4b9d8047fcfa0def2067fcae8a3a4077f81cbc7ae077fe5e3a2d6087fd4b4c59a097ffafb91aa0b7fa9fa82cd0b7f93bd84d80b7fbbd3c4e10b7fdbeb8b810c7fcaa4af8c0c7fd492fbc70c7fb0f0f18a0e7fa596ed8d0e7fd9d18baf0e7fc1a6abbb0e7fd9dfd5990f7ff28cd9ee0f7f01000000b817b4108407ec0e8407d00f'
            exit 0
            ;;
    esac
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

    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage sync-gateways downstream app --dry-run --json \
        > "$proof_root/blob-storage-sync-dry-run.json"
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage fund downstream app --cycles 900 --dry-run --json \
        > "$proof_root/blob-storage-fund-dry-run.json"
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage status downstream app --json \
        > "$proof_root/blob-storage-status-before.json"
    set +e
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage status downstream app --check-ready --json \
        > "$proof_root/blob-storage-status-check-ready-blocked.json" \
        2> "$proof_root/blob-storage-status-check-ready-blocked.err"
    check_ready_exit=$?
    set -e
    if [ "$check_ready_exit" -ne 4 ]; then
        echo "expected blob-storage check-ready blocked status to exit 4, got $check_ready_exit" >&2
        sed -n '1,220p' "$proof_root/blob-storage-status-check-ready-blocked.json" >&2
        sed -n '1,80p' "$proof_root/blob-storage-status-check-ready-blocked.err" >&2
        exit 1
    fi
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage sync-gateways downstream app --json \
        > "$proof_root/blob-storage-sync-live.json"
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage fund downstream app --cycles 900 --json \
        > "$proof_root/blob-storage-fund-live.json"
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage status downstream app --json \
        > "$proof_root/blob-storage-status-after.json"
    "$runner" --environment fixture --icp "$fake_icp" \
        blob-storage status downstream app --check-ready --json \
        > "$proof_root/blob-storage-status-check-ready-ready.json" \
        2> "$proof_root/blob-storage-status-check-ready-ready.err"
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
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage sync dry-run JSON kind" \
        '"kind": "blob_storage_sync_gateways_result"' \
        "$proof_root/blob-storage-sync-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage sync dry-run marker" \
        '"dry_run": true' \
        "$proof_root/blob-storage-sync-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage sync mode from Candid" \
        '"mode": "update"' \
        "$proof_root/blob-storage-sync-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage sync resolved target" \
        '"canister_id": "rrkah-fqaaa-aaaaa-aaaaq-cai"' \
        "$proof_root/blob-storage-sync-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage fund dry-run JSON kind" \
        '"kind": "blob_storage_fund_result"' \
        "$proof_root/blob-storage-fund-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage fund dry-run requested cycles" \
        '"requested_cycles": "900"' \
        "$proof_root/blob-storage-fund-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage fund dry-run command" \
        '_immutableObjectStorageFundFromProjectCycles (900 : nat)' \
        "$proof_root/blob-storage-fund-dry-run.json" \
        '1,200p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage fixture status JSON kind" \
        '"kind": "blob_storage_status"' \
        "$proof_root/blob-storage-status-before.json" \
        '1,220p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage fixture initial status to be blocked" \
        '"ready_for_upload": false' \
        "$proof_root/blob-storage-status-before.json" \
        '1,220p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage check-ready blocked output to preserve status JSON" \
        '"kind": "blob_storage_status"' \
        "$proof_root/blob-storage-status-check-ready-blocked.json" \
        '1,220p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage check-ready blocked output to show not ready" \
        '"ready_for_upload": false' \
        "$proof_root/blob-storage-status-check-ready-blocked.json" \
        '1,220p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage check-ready blocked stderr to explain blocker codes" \
        'readiness check failed: state=blocked; blockers=gateway_principals_empty,cashier_balance_below_min' \
        "$proof_root/blob-storage-status-check-ready-blocked.err" \
        '1,80p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage live sync JSON kind" \
        '"kind": "blob_storage_sync_gateways_result"' \
        "$proof_root/blob-storage-sync-live.json" \
        '1,260p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage live sync marker" \
        '"dry_run": false' \
        "$proof_root/blob-storage-sync-live.json" \
        '1,260p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage sync post-status gateway count" \
        '"principal_count": 1' \
        "$proof_root/blob-storage-sync-live.json" \
        '1,260p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage live fund JSON kind" \
        '"kind": "blob_storage_fund_result"' \
        "$proof_root/blob-storage-fund-live.json" \
        '1,260p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage live fund attached cycles" \
        '"attached_cycles": "900"' \
        "$proof_root/blob-storage-fund-live.json" \
        '1,260p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage live fund post-status readiness" \
        '"ready_for_upload": true' \
        "$proof_root/blob-storage-fund-live.json" \
        '1,260p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage final status readiness" \
        '"ready_for_upload": true' \
        "$proof_root/blob-storage-status-after.json" \
        '1,220p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage check-ready ready status" \
        '"ready_for_upload": true' \
        "$proof_root/blob-storage-status-check-ready-ready.json" \
        '1,220p'
    [ ! -s "$proof_root/blob-storage-status-check-ready-ready.err" ] || {
        echo "expected $proof_label blob-storage ready check to leave stderr empty" >&2
        sed -n '1,80p' "$proof_root/blob-storage-status-check-ready-ready.err" >&2
        exit 1
    }

    [ ! -s "$proof_root/blob-storage-status-json.out" ] || {
        echo "expected $proof_label blob-storage JSON failure to leave stdout empty" >&2
        sed -n '1,160p' "$proof_root/blob-storage-status-json.out" >&2
        exit 1
    }

    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error to include schema_version" \
        '"schema_version": 1' \
        "$proof_root/blob-storage-status-json.err" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error kind" \
        '"kind": "blob_storage_error"' \
        "$proof_root/blob-storage-status-json.err" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error target input" \
        '"input": "app"' \
        "$proof_root/blob-storage-status-json.err" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error code" \
        '"code": "target_resolution_failed"' \
        "$proof_root/blob-storage-status-json.err" \
        '1,160p'
    assert_blob_storage_cli_file_contains \
        "$proof_label" \
        "blob-storage JSON error exit code" \
        '"exit_code": 1' \
        "$proof_root/blob-storage-status-json.err" \
        '1,160p'
}
