// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{
    cdk::types::Principal,
    dto::{
        canister::CanisterInfo,
        env::EnvSnapshotResponse,
        page::{Page, PageRequest},
        state::{AppStateResponse, SubnetStateResponse},
    },
    ids::{CanisterRole, SubnetRole},
    protocol,
};
use canic_testkit::pic::Pic;

// Query the authoritative root subnet registry once and unwrap the canonical response shape.
fn query_subnet_registry(
    pic: &Pic,
    root_id: Principal,
) -> Vec<canic::dto::topology::SubnetRegistryEntry> {
    let registry: Result<canic::dto::topology::SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry transport");
    registry.expect("query registry application").0
}

/// Assert that the registry contains the expected roles with the expected parents.
pub fn assert_registry_parents(
    pic: &Pic,
    root_id: Principal,
    expected: &[(CanisterRole, Option<Principal>)],
) {
    let registry = query_subnet_registry(pic, root_id);

    for (role, expected_parent) in expected {
        let entry = registry
            .iter()
            .find(|entry| &entry.role == role)
            .unwrap_or_else(|| panic!("missing {role} entry in registry"));

        assert_eq!(
            entry.record.parent_pid, *expected_parent,
            "unexpected parent for {role}"
        );
    }
}

/// Assert that a child canister exposes a correct EnvSnapshotResponse.
pub fn assert_child_env(
    pic: &Pic,
    child_pid: Principal,
    role: CanisterRole,
    expected_parent_id: Principal,
    root_id: Principal,
) {
    let env: Result<EnvSnapshotResponse, canic::Error> = pic
        .query_call(child_pid, protocol::CANIC_ENV, ())
        .expect("query env transport");
    let env = env.expect("query env application");

    assert_eq!(
        env.canister_role,
        Some(role.clone()),
        "env canister role for {role}"
    );
    assert_eq!(
        env.parent_pid,
        Some(expected_parent_id),
        "env parent for {role}"
    );
    assert_eq!(env.root_pid, Some(root_id), "env root for {role}");
    assert_eq!(
        env.prime_root_pid,
        Some(root_id),
        "env prime root for {role}"
    );
    assert_eq!(
        env.subnet_role,
        Some(SubnetRole::PRIME),
        "env subnet role for {role}"
    );
    assert!(
        env.subnet_pid.is_some(),
        "env subnet pid should be set for {role}"
    );
}

/// Assert that every registered child exposes env fields matching the registry.
pub fn assert_child_envs_match_registry(pic: &Pic, root_id: Principal) {
    let registry = query_subnet_registry(pic, root_id);

    for entry in registry {
        if entry.role.is_root() || entry.role.is_wasm_store() {
            continue;
        }

        let expected_parent_id = entry.record.parent_pid.unwrap_or_else(|| {
            panic!(
                "registered non-root canister {} ({}) must have a parent",
                entry.pid, entry.role
            )
        });

        assert_child_env(pic, entry.pid, entry.role, expected_parent_id, root_id);
    }
}

/// Assert that the CANIC_CANISTER_CHILDREN endpoint matches the registry.
pub fn assert_children_match_registry(pic: &Pic, root_id: Principal) {
    // 1. Query authoritative registry
    let registry = query_subnet_registry(pic, root_id);

    // 2. Build expected children from registry (topology-only)
    let mut expected: Vec<CanisterInfo> = registry
        .iter()
        .filter(|entry| entry.record.parent_pid == Some(root_id))
        .map(|entry| CanisterInfo {
            pid: entry.pid,
            role: entry.role.clone(),
            parent_pid: entry.record.parent_pid,
            module_hash: None, // ignored for topology comparison
            created_at: 0,     // ignored for topology comparison
        })
        .collect();

    assert!(
        !expected.is_empty(),
        "registry should contain root children"
    );

    // 3. Query children endpoint
    let page: Result<Page<CanisterInfo>, canic::Error> = pic
        .query_call(
            root_id,
            protocol::CANIC_CANISTER_CHILDREN,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("query canister children transport");
    let mut page = page.expect("query canister children application");

    // 4. Normalize actual entries (ignore lifecycle metadata)
    for entry in &mut page.entries {
        entry.module_hash = None;
        entry.created_at = 0;
    }

    // 5. Normalize ordering (endpoint order is not significant)
    expected.sort_by(|a, b| a.role.cmp(&b.role));
    page.entries.sort_by(|a, b| a.role.cmp(&b.role));

    // 6. Assert invariants
    assert_eq!(page.total, expected.len() as u64, "reported total mismatch");

    assert_eq!(
        page.entries, expected,
        "child list from endpoint must match registry"
    );
}

/// Assert that root serves state snapshots and ordinary children do not export them.
pub fn assert_state_endpoints_are_root_only(pic: &Pic, root_id: Principal, child_pid: Principal) {
    let app_state: Result<AppStateResponse, canic::Error> = pic
        .query_call(root_id, protocol::CANIC_APP_STATE, ())
        .expect("root app state transport");
    app_state.expect("root app state application");

    let subnet_state: Result<SubnetStateResponse, canic::Error> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_STATE, ())
        .expect("root subnet state transport");
    subnet_state.expect("root subnet state application");

    let child_app_state: Result<Result<AppStateResponse, canic::Error>, canic::Error> =
        pic.query_call(child_pid, protocol::CANIC_APP_STATE, ());
    let Err(err) = child_app_state else {
        panic!("child app state endpoint should be absent")
    };
    assert_missing_method(&err, protocol::CANIC_APP_STATE);

    let child_subnet_state: Result<Result<SubnetStateResponse, canic::Error>, canic::Error> =
        pic.query_call(child_pid, protocol::CANIC_SUBNET_STATE, ());
    let Err(err) = child_subnet_state else {
        panic!("child subnet state endpoint should be absent")
    };
    assert_missing_method(&err, protocol::CANIC_SUBNET_STATE);
}

// Match PocketIC missing-method failures without depending on one exact transport string.
fn assert_missing_method(err: &canic::Error, method: &str) {
    let message = err.message.as_str();

    assert!(
        message.contains(method),
        "missing-method error should mention {method}: {message}"
    );
    assert!(
        message.contains("not found")
            || message.contains("has no method")
            || message.contains("has no query method")
            || message.contains("has no update method")
            || message.contains("unknown method")
            || message.contains("did not find method"),
        "expected missing-method transport failure for {method}, got: {message}"
    );
}
