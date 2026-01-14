use canic_core::{
    domain::policy::topology::{TopologyPolicy, TopologyPolicyError},
    ids::CanisterRole,
    ops::storage::{
        CanisterRecord,
        directory::app::AppDirectoryData,
        directory::subnet::SubnetDirectoryData,
        registry::subnet::SubnetRegistryData,
    },
};

#[test]
fn topology_invariants_live_in_policy() {
    let _guard = crate::lock();

    let toml = r#"
        app_directory = ["alpha"]

        [subnets.prime.canisters.alpha]
        kind = "singleton"
    "#;

    canic_core::init_config(toml).expect("init config");

    let role = CanisterRole::new("alpha");
    let registry_data = SubnetRegistryData {
        entries: vec![(
            crate::p(30),
            CanisterRecord {
                role: role.clone(),
                parent_pid: None,
                module_hash: None,
                created_at: 1,
            },
        )],
    };

    let app_data = AppDirectoryData {
        entries: Vec::new(),
    };
    let subnet_data = SubnetDirectoryData {
        entries: Vec::new(),
    };

    let err = TopologyPolicy::assert_directories_match_registry(
        &registry_data,
        &app_data,
        &subnet_data,
    )
    .expect_err("policy should detect app directory divergence");

    assert!(matches!(err, TopologyPolicyError::AppDirectoryDiverged));
}
