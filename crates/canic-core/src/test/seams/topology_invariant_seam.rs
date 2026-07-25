// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    config::schema::CanisterKind,
    domain::policy::pure::topology::{TopologyPolicy, TopologyPolicyError},
    ids::CanisterRole,
    model::topology::{TopologyDirectoryEntry, TopologyEntry, TopologyRegistry},
    test::{
        config::ConfigTestBuilder,
        seams::{lock, p},
    },
};

#[test]
fn topology_invariants_live_in_policy() {
    let _guard = lock();

    let _config = ConfigTestBuilder::new()
        .with_default_canister_kind("alpha", CanisterKind::Service)
        .install();

    let role = CanisterRole::new("alpha");
    let registry_data = TopologyRegistry {
        entries: vec![TopologyEntry {
            pid: p(30),
            role,
            parent_pid: None,
            module_hash: None,
        }],
    };

    let mismatched = vec![TopologyDirectoryEntry {
        role: CanisterRole::new("beta"),
        pid: p(30),
    }];

    let err =
        TopologyPolicy::assert_directory_consistent_with_registry(&registry_data, &mismatched)
            .expect_err("policy should detect Directory divergence");

    std::assert_matches!(err, TopologyPolicyError::DirectoryRoleMismatch { .. });
}
