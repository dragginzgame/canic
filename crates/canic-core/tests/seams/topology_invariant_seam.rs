use canic_core::{
    domain::policy::topology::{TopologyPolicy, TopologyPolicyError},
    ids::CanisterRole,
    ops::storage::{
        directory::app::AppDirectorySnapshot,
        directory::subnet::SubnetDirectorySnapshot,
        registry::subnet::{CanisterEntrySnapshot, SubnetRegistrySnapshot},
    },
};

#[test]
fn topology_invariants_live_in_policy() {
    let _guard = crate::lock();

    let toml = r#"
        app_directory = ["alpha"]

        [subnets.prime.canisters.alpha]
        cardinality = "single"
    "#;

    canic_core::init_config(toml).expect("init config");

    let role = CanisterRole::new("alpha");
    let registry_snapshot = SubnetRegistrySnapshot {
        entries: vec![(
            crate::p(30),
            CanisterEntrySnapshot {
                role: role.clone(),
                parent_pid: None,
                module_hash: None,
                created_at: 1,
            },
        )],
    };

    let app_snapshot = AppDirectorySnapshot { entries: Vec::new() };
    let subnet_snapshot = SubnetDirectorySnapshot { entries: Vec::new() };

    let err = TopologyPolicy::assert_directories_match_registry(
        &registry_snapshot,
        &app_snapshot,
        &subnet_snapshot,
    )
    .expect_err("policy should detect app directory divergence");

    assert!(matches!(err, TopologyPolicyError::AppDirectoryDiverged));
}
