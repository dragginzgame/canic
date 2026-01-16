use crate::{
    config::schema::CanisterKind,
    domain::policy::topology::{TopologyPolicy, TopologyPolicyError},
    ids::CanisterRole,
    ops::storage::{CanisterRecord, registry::subnet::SubnetRegistryData},
    test::{
        config::ConfigTestBuilder,
        seams::{lock, p},
    },
};

#[test]
fn topology_invariants_live_in_policy() {
    let _guard = lock();

    let _config = ConfigTestBuilder::new()
        .with_app_directory("alpha")
        .with_prime_canister_kind("alpha", CanisterKind::Node)
        .install();

    let role = CanisterRole::new("alpha");
    let registry_data = SubnetRegistryData {
        entries: vec![(
            p(30),
            CanisterRecord {
                role,
                parent_pid: None,
                module_hash: None,
                created_at: 1,
            },
        )],
    };

    let mismatched = vec![(CanisterRole::new("beta"), p(30))];

    let err =
        TopologyPolicy::assert_directory_consistent_with_registry(&registry_data, &mismatched)
            .expect_err("policy should detect directory divergence");

    assert!(matches!(
        err,
        TopologyPolicyError::DirectoryRoleMismatch { .. }
    ));
}
