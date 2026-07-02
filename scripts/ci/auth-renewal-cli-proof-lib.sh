#!/usr/bin/env bash

# Shared CLI surface proofing for the hard-cut auth renewal command family.
# The retained operator surface is `auth renewal status`; this helper also
# asserts removed bridge/provisioner commands do not reappear in help output.

AUTH_RENEWAL_PROOF_ISSUER="rrkah-fqaaa-aaaaa-aaaaq-cai"

prepare_auth_renewal_cli_surface_fixture() {
    local downstream_root="$1"

    mkdir -p \
        "$downstream_root/.canic/fixture/deployments" \
        "$downstream_root/.icp/fixture/canisters/app" \
        "$downstream_root/.icp/fixture/canisters/root" \
        "$downstream_root/fleets/downstream/app" \
        "$downstream_root/fleets/downstream/root"

    cat > "$downstream_root/icp.yaml" <<'EOF'
canisters:
  - name: root
  - name: app

networks:
  - name: local
    mode: managed
    gateway:
      bind: 127.0.0.1
      port: 8001

environments:
  - name: downstream
    network: local
    canisters: [root, app]
EOF

    cat > "$downstream_root/fleets/downstream/canic.toml" <<'EOF'
controllers = []
app_index = ["app"]

[fleet]
name = "downstream"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "service"
EOF

    cat > "$downstream_root/.canic/fixture/deployments/downstream.json" <<EOF
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
  "workspace_root": "$downstream_root",
  "icp_root": "$downstream_root",
  "config_path": "$downstream_root/fleets/downstream/canic.toml",
  "release_set_manifest_path": "$downstream_root/.icp/fixture/canisters/root/release-set.json"
}
EOF

    cat > "$downstream_root/fleets/downstream/root/Cargo.toml" <<'EOF'
[package]
name = "downstream-root"
version = { workspace = true }
edition = "2024"

[package.metadata.canic]
fleet = "downstream"
role = "root"
EOF

    cat > "$downstream_root/fleets/downstream/app/Cargo.toml" <<'EOF'
[package]
name = "downstream-app"
version = { workspace = true }
edition = "2024"

[package.metadata.canic]
fleet = "downstream"
role = "app"
EOF

    cat > "$downstream_root/.icp/fixture/canisters/root/root.did" <<'EOF'
service : {
  canic_ready : () -> (bool) query;
  canic_subnet_registry : () -> () query;
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

run_auth_renewal_cli_surface_probe_commands() {
    local runner="$1"
    local proof_root="$2"
    local fake_icp="$3"

    "$runner" auth help > "$proof_root/auth-renewal-help.out"
    "$runner" --network fixture --icp "$fake_icp" \
        auth renewal status downstream --issuer "$AUTH_RENEWAL_PROOF_ISSUER" --json \
        > "$proof_root/auth-renewal-status-drift.json"
    "$runner" --network fixture --icp "$fake_icp" \
        medic deployment downstream --auth-renewal "$AUTH_RENEWAL_PROOF_ISSUER" \
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

assert_auth_renewal_cli_file_not_contains() {
    local proof_label="$1"
    local description="$2"
    local pattern="$3"
    local path="$4"
    local preview_range="$5"

    if grep -Fq -- "$pattern" "$path"; then
        echo "expected $proof_label $description" >&2
        sed -n "$preview_range" "$path" >&2
        exit 1
    fi
}

assert_auth_renewal_cli_surface_probe_outputs() {
    local proof_label="$1"
    local proof_root="$2"

    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to describe renewal workflows" \
        'Run delegated-auth operator workflows' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_not_contains \
        "$proof_label" \
        "auth renewal help to omit removed run-once bridge" \
        'run-once' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to list status" \
        'status' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_not_contains \
        "$proof_label" \
        "auth renewal help to omit removed provisioner commands" \
        'provisioner' \
        "$proof_root/auth-renewal-help.out" \
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
        'auth [warn] auth_renewal_drift_warn' \
        "$proof_root/auth-renewal-medic-drift.out" \
        '1,180p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal medic drift status" \
        'status=drift_detected' \
        "$proof_root/auth-renewal-medic-drift.out" \
        '1,180p'
}
