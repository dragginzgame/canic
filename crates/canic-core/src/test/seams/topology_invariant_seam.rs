// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    config::schema::CanisterKind,
    domain::policy::topology::{
        RegistryPolicyInput, TopologyPolicy, TopologyPolicyError, TopologyPolicyInput,
    },
    ids::CanisterRole,
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
    let registry_data = RegistryPolicyInput {
        entries: vec![TopologyPolicyInput {
            pid: p(30),
            role,
            parent_pid: None,
            module_hash: None,
        }],
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
