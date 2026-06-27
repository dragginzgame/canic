#!/usr/bin/env bash

AUTH_RENEWAL_PROOF_ISSUER="rrkah-fqaaa-aaaaa-aaaaq-cai"

prepare_auth_renewal_cli_fixture() {
    local downstream_root="$1"

    mkdir -p \
        "$downstream_root/.icp/fixture/canisters/app" \
        "$downstream_root/.icp/fixture/canisters/root"

    cat > "$downstream_root/.icp/fixture/canisters/root/root.did" <<'EOF'
service : {
  canic_subnet_registry : () -> () query;
  canic_delegation_renewal_work : () -> () query;
  canic_root_issuer_renewal_status : (record { issuer_pid : principal }) -> () query;
}
EOF

    cat > "$downstream_root/.icp/fixture/canisters/app/app.did" <<'EOF'
service : {
  get_blob_storage_status : (record { sync_gateway_principals : bool }) -> () query;
  "_immutableObjectStorageUpdateGatewayPrincipals" : () -> ();
  "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
  canic_active_delegation_proof_status : () -> () query;
}
EOF
}

run_auth_renewal_cli_probe_commands() {
    local runner="$1"
    local proof_root="$2"
    local fake_icp="$3"

    "$runner" auth help > "$proof_root/auth-renewal-help.out"
    "$runner" --network fixture --icp "$fake_icp" \
        auth renewal run-once downstream --json \
        > "$proof_root/auth-renewal-run-once-no-work.json"
    "$runner" --network fixture --icp "$fake_icp" \
        auth renewal status downstream --issuer "$AUTH_RENEWAL_PROOF_ISSUER" --json \
        > "$proof_root/auth-renewal-status-drift.json"
    "$runner" --network fixture --icp "$fake_icp" \
        info medic downstream --auth-renewal "$AUTH_RENEWAL_PROOF_ISSUER" \
        > "$proof_root/auth-renewal-medic-drift.out"
}

assert_auth_renewal_cli_file_contains() {
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

assert_auth_renewal_cli_probe_outputs() {
    local proof_label="$1"
    local proof_root="$2"

    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to describe renewal workflows" \
        'Run root-managed delegation proof renewal workflows' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to list run-once" \
        'run-once' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to list status" \
        'status' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal run-once JSON kind" \
        '"kind": "auth_renewal_run_once_result"' \
        "$proof_root/auth-renewal-run-once-no-work.json" \
        '1,180p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal run-once no-work status" \
        '"status": "no_work"' \
        "$proof_root/auth-renewal-run-once-no-work.json" \
        '1,180p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal status schema v2" \
        '"schema_version": 2' \
        "$proof_root/auth-renewal-status-drift.json" \
        '1,220p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal status JSON kind" \
        '"kind": "auth_renewal_status"' \
        "$proof_root/auth-renewal-status-drift.json" \
        '1,220p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal status drift status" \
        '"status": "drift_detected"' \
        "$proof_root/auth-renewal-status-drift.json" \
        '1,260p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal status issuer observation" \
        '"issuer_observation"' \
        "$proof_root/auth-renewal-status-drift.json" \
        '1,260p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal status drift flag" \
        '"drift_detected": true' \
        "$proof_root/auth-renewal-status-drift.json" \
        '1,260p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal medic warning" \
        'auth renewal [warn]' \
        "$proof_root/auth-renewal-medic-drift.out" \
        '1,180p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal medic drift status" \
        'status=drift_detected' \
        "$proof_root/auth-renewal-medic-drift.out" \
        '1,180p'
}
