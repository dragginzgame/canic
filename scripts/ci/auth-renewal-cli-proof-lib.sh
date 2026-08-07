#!/usr/bin/env bash

# Shared CLI surface proofing for the maintained auth renewal status command.

AUTH_RENEWAL_PROOF_ISSUER="rrkah-fqaaa-aaaaa-aaaaq-cai"

prepare_auth_renewal_cli_surface_fixture() {
    local downstream_root="$1"

    mkdir -p \
        "$downstream_root/.icp/fixture/canisters/app" \
        "$downstream_root/apps/downstream/app" \
        "$downstream_root/apps/downstream/root"

    cat > "$downstream_root/icp.yaml" <<'EOF'
canisters:
  - name: root
  - name: app

environments:
  - name: fixture
    network: ic
    canisters: [root, app]
EOF

    cat > "$downstream_root/apps/downstream/canic.toml" <<'EOF'
[app]
name = "downstream"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[component_specs.app]
component_role = "app"
maximum_instances = 1
EOF

    cat > "$downstream_root/apps/downstream/root/Cargo.toml" <<'EOF'
[package]
name = "downstream-root"
version = { workspace = true }
edition = "2024"

[package.metadata.canic]
app = "downstream"
role = "root"
EOF

    cat > "$downstream_root/apps/downstream/app/Cargo.toml" <<'EOF'
[package]
name = "downstream-app"
version = { workspace = true }
edition = "2024"

[package.metadata.canic]
app = "downstream"
role = "app"
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
    local status_exit=0
    local medic_exit=0

    "$runner" auth --help > "$proof_root/auth-renewal-help.out"
    set +e
    "$runner" --environment fixture --icp "$fake_icp" \
        auth renewal status downstream --issuer "$AUTH_RENEWAL_PROOF_ISSUER" --json \
        > "$proof_root/auth-renewal-status-coordinator-fence.out" \
        2> "$proof_root/auth-renewal-status-coordinator-fence.err"
    status_exit=$?
    "$runner" --environment fixture --icp "$fake_icp" \
        medic fleet downstream --auth-renewal "$AUTH_RENEWAL_PROOF_ISSUER" \
        > "$proof_root/auth-renewal-medic-coordinator-fence.out"
    medic_exit=$?
    set -e
    if [ "$status_exit" -ne 1 ]; then
        echo "expected Coordinator-anchored auth renewal status to exit 1, got $status_exit" >&2
        exit 1
    fi
    if [ "$medic_exit" -ne 1 ]; then
        echo "expected Coordinator-anchored auth-renewal medic to exit 1, got $medic_exit" >&2
        exit 1
    fi
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

assert_auth_renewal_cli_surface_probe_outputs() {
    local proof_label="$1"
    local proof_root="$2"

    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to describe renewal workflows" \
        'Run delegated-auth operator workflows' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal help to list status" \
        'status' \
        "$proof_root/auth-renewal-help.out" \
        '1,160p'
    [ ! -s "$proof_root/auth-renewal-status-coordinator-fence.out" ] || {
        echo "expected $proof_label auth renewal Coordinator fence to leave stdout empty" >&2
        sed -n '1,160p' "$proof_root/auth-renewal-status-coordinator-fence.out" >&2
        exit 1
    }
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal status to identify the removed resolver" \
        'Coordinator-anchored' \
        "$proof_root/auth-renewal-status-coordinator-fence.err" \
        '1,160p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal medic to report unavailable current topology" \
        'fleet_registry_unavailable' \
        "$proof_root/auth-renewal-medic-coordinator-fence.out" \
        '1,180p'
    assert_auth_renewal_cli_file_contains \
        "$proof_label" \
        "auth renewal medic to identify the removed resolver" \
        'Coordinator-anchored' \
        "$proof_root/auth-renewal-medic-coordinator-fence.out" \
        '1,180p'
}
