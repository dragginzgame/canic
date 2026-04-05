// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{
    cdk::types::Principal,
    dto::{
        canister::CanisterInfo,
        env::EnvSnapshotResponse,
        page::{Page, PageRequest},
        state::{AppStateResponse, SubnetStateResponse},
        topology::DirectoryEntryResponse,
    },
    ids::{CanisterRole, SubnetRole},
    protocol,
};
use canic_testkit::pic::Pic;
use std::collections::HashMap;

/// Assert that the registry contains the expected roles with the expected parents.
pub fn assert_registry_parents(
    pic: &Pic,
    root_id: Principal,
    expected: &[(CanisterRole, Option<Principal>)],
) {
    let registry: Result<canic::dto::topology::SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry transport");
    let registry = registry.expect("query registry application").0;

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

/// Look up one canister principal by role in the root subnet registry.
pub fn registry_pid_for_role(pic: &Pic, root_id: Principal, role: &CanisterRole) -> Principal {
    let registry: Result<canic::dto::topology::SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry transport");
    let registry = registry.expect("query registry application").0;

    registry
        .iter()
        .find(|entry| &entry.role == role)
        .map_or_else(
            || panic!("missing {role} entry in registry"),
            |entry| entry.pid,
        )
}

/// Assert that a child canister exposes a correct EnvSnapshotResponse.
pub fn assert_child_env(pic: &Pic, child_pid: Principal, role: CanisterRole, root_id: Principal) {
    let env: Result<EnvSnapshotResponse, canic::Error> = pic
        .query_call(child_pid, protocol::CANIC_ENV, ())
        .expect("query env transport");
    let env = env.expect("query env application");

    assert_eq!(
        env.canister_role,
        Some(role.clone()),
        "env canister role for {role}"
    );
    assert_eq!(env.parent_pid, Some(root_id), "env parent for {role}");
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

/// Assert that the root directories are authoritative and child views stay rooted in them.
pub fn assert_directories_consistent(
    pic: &Pic,
    root_id: Principal,
    subnet_directory: &HashMap<CanisterRole, Principal>,
) {
    let root_app_dir = query_directory(pic, root_id, protocol::CANIC_APP_DIRECTORY, "root app");
    let root_subnet_dir = query_directory(
        pic,
        root_id,
        protocol::CANIC_SUBNET_DIRECTORY,
        "root subnet",
    );

    assert_root_directories_match_snapshot(&root_app_dir, &root_subnet_dir, subnet_directory);

    for (role, pid) in subnet_directory.iter().filter(|(r, _)| !r.is_root()) {
        assert_child_directories_resolve_to_root(
            pic,
            *pid,
            role,
            *pid,
            &root_app_dir,
            &root_subnet_dir,
        );
    }
}

// Query one directory endpoint from one canister with the canonical page shape.
fn query_directory(
    pic: &Pic,
    canister_id: Principal,
    method: &str,
    label: &str,
) -> Page<DirectoryEntryResponse> {
    let response: Result<Page<DirectoryEntryResponse>, canic::Error> = pic
        .query_call(
            canister_id,
            method,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .unwrap_or_else(|_| panic!("query {label} directory transport"));
    response.unwrap_or_else(|_| panic!("query {label} directory application"))
}

// Normalize directory entries into stable `(role, pid)` tuples for comparisons.
fn normalize_directory_entries(
    entries: &[DirectoryEntryResponse],
) -> Vec<(CanisterRole, Principal)> {
    let mut normalized = entries
        .iter()
        .map(|entry| (entry.role.clone(), entry.pid))
        .collect::<Vec<_>>();
    normalized.sort_by(|a, b| a.0.cmp(&b.0));
    normalized
}

// Assert that root-owned directory surfaces line up with the harness snapshot.
fn assert_root_directories_match_snapshot(
    root_app_dir: &Page<DirectoryEntryResponse>,
    root_subnet_dir: &Page<DirectoryEntryResponse>,
    subnet_directory: &HashMap<CanisterRole, Principal>,
) {
    let expected_subnet_dir = normalize_directory_entries(
        &subnet_directory
            .iter()
            .map(|(role, pid)| DirectoryEntryResponse {
                role: role.clone(),
                pid: *pid,
            })
            .collect::<Vec<_>>(),
    );
    let actual_root_subnet_dir = normalize_directory_entries(&root_subnet_dir.entries);

    assert_eq!(
        actual_root_subnet_dir, expected_subnet_dir,
        "root subnet directory must match the harness subnet directory snapshot"
    );

    for entry in &root_app_dir.entries {
        let expected_pid = subnet_directory.get(&entry.role).unwrap_or_else(|| {
            panic!(
                "root app directory role {} missing from harness",
                entry.role
            )
        });
        assert_eq!(
            entry.pid, *expected_pid,
            "root app directory pid mismatch for {}",
            entry.role
        );
    }
}

// Assert that one child's partial directory views stay rooted in the authoritative root entries.
fn assert_child_directories_resolve_to_root(
    pic: &Pic,
    child_pid: Principal,
    role: &CanisterRole,
    expected_pid: Principal,
    root_app_dir: &Page<DirectoryEntryResponse>,
    root_subnet_dir: &Page<DirectoryEntryResponse>,
) {
    let app_dir = query_directory(pic, child_pid, protocol::CANIC_APP_DIRECTORY, "child app");
    let subnet_dir = query_directory(
        pic,
        child_pid,
        protocol::CANIC_SUBNET_DIRECTORY,
        "child subnet",
    );

    for entry in &app_dir.entries {
        assert!(
            root_app_dir.entries.contains(entry),
            "app directory entry {entry:?} for {role} must exist in the root app directory",
        );
    }

    for entry in &subnet_dir.entries {
        assert!(
            root_subnet_dir.entries.contains(entry),
            "subnet directory entry {entry:?} for {role} must exist in the root subnet directory",
        );
    }

    if root_app_dir.entries.iter().any(|entry| entry.role == *role) {
        assert!(
            app_dir
                .entries
                .iter()
                .any(|entry| entry.role == *role && entry.pid == expected_pid),
            "app directory for {role} must still resolve the child itself",
        );
    }

    assert!(
        subnet_dir
            .entries
            .iter()
            .any(|entry| entry.role == *role && entry.pid == expected_pid),
        "subnet directory for {role} must still resolve the child itself",
    );
}

/// Assert that the CANIC_CANISTER_CHILDREN endpoint matches the registry.
pub fn assert_children_match_registry(pic: &Pic, root_id: Principal) {
    // 1. Query authoritative registry
    let registry: Result<canic::dto::topology::SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry transport");
    let registry = registry.expect("query registry application").0;

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
