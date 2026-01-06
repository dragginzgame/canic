use canic::{
    cdk::types::Principal,
    core::{
        dto::{
            canister::CanisterSummaryView,
            env::EnvView,
            page::{Page, PageRequest},
            topology::{DirectoryEntryView, SubnetRegistryEntryView},
        },
        ids::{CanisterRole, SubnetRole},
        protocol,
    },
};
use canic_testkit::pic::Pic;
use std::collections::HashMap;

/// Assert that the registry contains the expected roles with the expected parents.
pub fn assert_registry_parents(
    pic: &Pic,
    root_id: Principal,
    expected: &[(CanisterRole, Option<Principal>)],
) {
    let registry: Vec<SubnetRegistryEntryView> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry");

    for (role, expected_parent) in expected {
        let entry = registry
            .iter()
            .find(|entry| &entry.role == role)
            .unwrap_or_else(|| panic!("missing {role} entry in registry"));

        assert_eq!(
            entry.entry.parent_pid, *expected_parent,
            "unexpected parent for {role}"
        );
    }
}

/// Assert that a child canister exposes a correct EnvView.
pub fn assert_child_env(pic: &Pic, child_pid: Principal, role: CanisterRole, root_id: Principal) {
    let env: EnvView = pic
        .query_call(child_pid, protocol::CANIC_ENV, ())
        .expect("query env");

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

/// Assert that app and subnet directories are identical across all canisters.
pub fn assert_directories_consistent(
    pic: &Pic,
    root_id: Principal,
    subnet_directory: &HashMap<CanisterRole, Principal>,
) {
    let root_app_dir: Page<DirectoryEntryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_APP_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("root app directory");

    let root_subnet_dir: Page<DirectoryEntryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_SUBNET_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("root subnet directory");

    for (role, pid) in subnet_directory.iter().filter(|(r, _)| !r.is_root()) {
        let app_dir: Page<DirectoryEntryView> = pic
            .query_call(
                *pid,
                protocol::CANIC_APP_DIRECTORY,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("child app directory");

        let subnet_dir: Page<DirectoryEntryView> = pic
            .query_call(
                *pid,
                protocol::CANIC_SUBNET_DIRECTORY,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("child subnet directory");

        assert_eq!(
            app_dir.entries,
            root_app_dir.entries,
            "app directory mismatch for {role} (child={}, root={})",
            app_dir.entries.len(),
            root_app_dir.entries.len(),
        );

        assert_eq!(
            subnet_dir.entries,
            root_subnet_dir.entries,
            "subnet directory mismatch for {role} (child={}, root={})",
            subnet_dir.entries.len(),
            root_subnet_dir.entries.len(),
        );
    }
}

/// Assert that the CANIC_CANISTER_CHILDREN endpoint matches the registry.
pub fn assert_children_match_registry(pic: &Pic, root_id: Principal) {
    let registry: Vec<SubnetRegistryEntryView> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry");

    let mut expected: Vec<CanisterSummaryView> = registry
        .iter()
        .filter(|entry| entry.entry.parent_pid == Some(root_id))
        .map(|entry| CanisterSummaryView {
            role: entry.role.clone(),
            parent_pid: entry.entry.parent_pid,
        })
        .collect();

    assert!(
        !expected.is_empty(),
        "registry should contain root children"
    );

    let mut page: Page<CanisterSummaryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_CANISTER_CHILDREN,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("query canister children");

    expected.sort_by(|a, b| a.role.cmp(&b.role));
    page.entries.sort_by(|a, b| a.role.cmp(&b.role));

    assert_eq!(page.total, expected.len() as u64, "reported total mismatch");
    assert_eq!(
        page.entries, expected,
        "child list from endpoint must match registry"
    );
}
