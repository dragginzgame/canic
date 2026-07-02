#!/usr/bin/env bash

prepare_blob_storage_cli_fixture() {
    local downstream_root="$1"

    mkdir -p \
        "$downstream_root/.canic/fixture/deployments" \
        "$downstream_root/.icp/fixture/canisters/app" \
        "$downstream_root/.icp/fixture/canisters/root"

    cat > "$downstream_root/.canic/fixture/deployments/downstream.json" <<'EOF'
{
  "schema_version": 2,
  "deployment_name": "downstream",
  "fleet_template": "downstream",
  "created_at_unix_secs": 1,
  "updated_at_unix_secs": 1,
  "network": "fixture",
  "root_target": "root",
  "root_canister_id": "ryjl3-tyaaa-aaaaa-aaaba-cai",
  "root_verification": "not_verified",
  "root_build_target": "root",
  "workspace_root": ".",
  "icp_root": ".",
  "config_path": "fleets/downstream/canic.toml",
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

for arg in "$@"; do
    if [ "$arg" = "--version" ]; then
        echo "icp-cli 1.0.0"
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
            cat <<'JSON'
{"Ok":[{"pid":"ryjl3-tyaaa-aaaaa-aaaba-cai","role":"root","record":{"pid":"ryjl3-tyaaa-aaaaa-aaaba-cai","role":"root","kind":"root","parent_pid":null,"module_hash":null}},{"pid":"rrkah-fqaaa-aaaaa-aaaaq-cai","role":"app","kind":"singleton","record":{"pid":"rrkah-fqaaa-aaaaa-aaaaq-cai","role":"app","parent_pid":["ryjl3-tyaaa-aaaaa-aaaba-cai"],"module_hash":null}}]}
JSON
            exit 0
            ;;
        canic_root_issuer_renewal_status)
            cat <<'JSON'
{"template":{"enabled":true,"cert_ttl_ns":"300000000000"},"state":{"last_installed_cert_hash":["0303030303030303030303030303030303030303030303030303030303030303"],"last_outcome":"Installed","consecutive_failures":0,"last_installed_expires_at_ns":["1620329000000000000"],"last_installed_refresh_after_ns":["1620328900000000000"],"next_attempt_after_ns":"1620328900000000000","active_attempt_id":null},"active_attempt":null}
JSON
            exit 0
            ;;
        canic_active_delegation_proof_status)
            cat <<'JSON'
{"status":"Valid","root_pid":["ryjl3-tyaaa-aaaaa-aaaba-cai"],"issuer_pid":["rrkah-fqaaa-aaaaa-aaaaq-cai"],"cert_hash":["0404040404040404040404040404040404040404040404040404040404040404"],"expires_at_ns":["1620329000000000000"],"refresh_after_ns":["1620328900000000000"]}
JSON
            exit 0
            ;;
        get_blob_storage_status)
            state="$(cat "$STATE_FILE")"
            if [ "$state" = "funded" ]; then
                cat <<'JSON'
{"Ok":{"payment_model":{"ProjectAsPaymentAccount":null},"cashier_canister_id":["ryjl3-tyaaa-aaaaa-aaaba-cai"],"payment_account":["rrkah-fqaaa-aaaaa-aaaaq-cai"],"cashier_balance":["1900"],"min_upload_balance":["500"],"target_upload_balance":["1000"],"project_cycles_reserve":["2000"],"project_cycles_available":"2100","gateway_principal_count":1,"last_gateway_principal_sync_at_ns":["123"],"gateway_principal_sync_action":{"SkippedReadOnlyStatus":null},"funding_status":{"NotNeeded":null},"ready":true,"blockers":[],"warnings":[]}}
JSON
            elif [ "$state" = "synced" ]; then
                cat <<'JSON'
{"Ok":{"payment_model":{"ProjectAsPaymentAccount":null},"cashier_canister_id":["ryjl3-tyaaa-aaaaa-aaaba-cai"],"payment_account":["rrkah-fqaaa-aaaaa-aaaaq-cai"],"cashier_balance":["100"],"min_upload_balance":["500"],"target_upload_balance":["1000"],"project_cycles_reserve":["2000"],"project_cycles_available":"3000","gateway_principal_count":1,"last_gateway_principal_sync_at_ns":["123"],"gateway_principal_sync_action":{"SkippedReadOnlyStatus":null},"funding_status":{"FundingRequired":{"requested_cycles":"900"}},"ready":false,"blockers":[{"InsufficientCashierBalance":null}],"warnings":[]}}
JSON
            else
                cat <<'JSON'
{"Ok":{"payment_model":{"ProjectAsPaymentAccount":null},"cashier_canister_id":["ryjl3-tyaaa-aaaaa-aaaba-cai"],"payment_account":["rrkah-fqaaa-aaaaa-aaaaq-cai"],"cashier_balance":["100"],"min_upload_balance":["500"],"target_upload_balance":["1000"],"project_cycles_reserve":["2000"],"project_cycles_available":"3000","gateway_principal_count":0,"last_gateway_principal_sync_at_ns":null,"gateway_principal_sync_action":{"SkippedReadOnlyStatus":null},"funding_status":{"FundingRequired":{"requested_cycles":"900"}},"ready":false,"blockers":[{"GatewayPrincipalsMissing":null},{"InsufficientCashierBalance":null}],"warnings":[]}}
JSON
            fi
            exit 0
            ;;
        _immutableObjectStorageUpdateGatewayPrincipals)
            printf 'synced\n' > "$STATE_FILE"
            echo '{}'
            exit 0
            ;;
        _immutableObjectStorageFundFromProjectCycles)
            printf 'funded\n' > "$STATE_FILE"
            cat <<'JSON'
{"Ok":{"requested_cycles":"900","attached_cycles":"900","project_cycles_before":"3000","project_cycles_after":"2100","reserve_cycles":"2000","cashier_total_after":"1900","skipped_reason":null}}
JSON
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

    "$runner" --network fixture --icp "$fake_icp" \
        blob-storage sync-gateways downstream app --dry-run --json \
        > "$proof_root/blob-storage-sync-dry-run.json"
    "$runner" --network fixture --icp "$fake_icp" \
        blob-storage fund downstream app --cycles 900 --dry-run --json \
        > "$proof_root/blob-storage-fund-dry-run.json"
    "$runner" --network fixture --icp "$fake_icp" \
        blob-storage status downstream app --json \
        > "$proof_root/blob-storage-status-before.json"
    set +e
    "$runner" --network fixture --icp "$fake_icp" \
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
    "$runner" --network fixture --icp "$fake_icp" \
        blob-storage sync-gateways downstream app --json \
        > "$proof_root/blob-storage-sync-live.json"
    "$runner" --network fixture --icp "$fake_icp" \
        blob-storage fund downstream app --cycles 900 --json \
        > "$proof_root/blob-storage-fund-live.json"
    "$runner" --network fixture --icp "$fake_icp" \
        blob-storage status downstream app --json \
        > "$proof_root/blob-storage-status-after.json"
    "$runner" --network fixture --icp "$fake_icp" \
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
