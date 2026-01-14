use canic_core::{
    ids::CanisterRole,
    ops::storage::{
        directory::subnet::{SubnetDirectoryData, SubnetDirectoryOps},
        registry::subnet::SubnetRegistryOps,
    },
    workflow::topology::directory::query::subnet_directory_pid_by_role,
};

#[test]
fn directory_addressing_prefers_directory_over_registry_duplicates() {
    let _guard = crate::lock();

    let role = CanisterRole::new("seam_directory_role");
    let root_pid = crate::p(10);
    let pid_a = crate::p(11);
    let pid_b = crate::p(12);

    let created_at = 1;
    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(pid_a, &role, root_pid, vec![], created_at)
        .expect("register first canister");
    SubnetRegistryOps::register_unchecked(pid_b, &role, root_pid, vec![], created_at)
        .expect("register second canister with same role");

    SubnetDirectoryOps::import(SubnetDirectoryData {
        entries: vec![(role.clone(), pid_b)],
    })
    .expect("import subnet directory");

    let resolved = subnet_directory_pid_by_role(role.clone()).expect("directory role missing");
    assert_eq!(resolved, pid_b);

    let duplicates = SubnetRegistryOps::snapshot()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();

    assert_eq!(duplicates, 2);
}
