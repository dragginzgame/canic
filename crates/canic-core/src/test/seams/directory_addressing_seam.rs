// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    ids::CanisterRole,
    ops::storage::{directory::subnet::SubnetDirectoryOps, registry::subnet::SubnetRegistryOps},
    storage::stable::directory::subnet::SubnetDirectoryRecord,
    test::seams::{lock, p},
    workflow::topology::directory::query::SubnetDirectoryQuery,
};

#[test]
fn directory_addressing_prefers_directory_over_registry_duplicates() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }
    SubnetDirectoryOps::import_allow_incomplete(SubnetDirectoryRecord {
        entries: Vec::new(),
    })
    .expect("clear subnet directory");

    let role = CanisterRole::new("seam_directory_role");
    let root_pid = p(10);
    let pid_a = p(11);
    let pid_b = p(12);

    let created_at = 1;
    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(pid_a, &role, root_pid, vec![], created_at)
        .expect("register first canister");
    SubnetRegistryOps::register_unchecked(pid_b, &role, root_pid, vec![], created_at)
        .expect("register second canister with same role");

    SubnetDirectoryOps::import_allow_incomplete(SubnetDirectoryRecord {
        entries: vec![(role.clone(), pid_b)],
    })
    .expect("import subnet directory");

    let resolved = SubnetDirectoryQuery::get(role.clone()).expect("directory role missing");
    assert_eq!(resolved, pid_b);

    let duplicates = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();

    assert_eq!(duplicates, 2);
}

#[test]
fn directory_addressing_does_not_fallback_to_registry() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }
    SubnetDirectoryOps::import_allow_incomplete(SubnetDirectoryRecord {
        entries: Vec::new(),
    })
    .expect("clear subnet directory");

    let role = CanisterRole::new("seam_directory_no_fallback");
    let root_pid = p(13);
    let pid = p(14);
    let created_at = 1;

    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(pid, &role, root_pid, vec![], created_at)
        .expect("register canister");

    let resolved = SubnetDirectoryQuery::get(role.clone());
    assert!(resolved.is_none());

    let registry_count = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();
    assert_eq!(registry_count, 1);
}
