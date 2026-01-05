use canic_core::{
    cdk::types::Cycles,
    config::schema::{CanisterCardinality, CanisterConfig, RandomnessConfig},
    domain::policy::topology::registry::{RegistryPolicy, RegistryPolicyError},
    ids::CanisterRole,
    ops::storage::registry::subnet::{
        CanisterEntrySnapshot, SubnetRegistryOps, SubnetRegistrySnapshot,
    },
};

fn single_canister_config() -> CanisterConfig {
    CanisterConfig {
        cardinality: CanisterCardinality::Single,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
    }
}

#[test]
fn registry_cardinality_policy_blocks_but_ops_allows() {
    let _guard = crate::lock();

    let role = CanisterRole::new("seam_registry_singleton");
    let existing_pid = crate::p(1);
    let root_pid = crate::p(2);

    let snapshot = SubnetRegistrySnapshot {
        entries: vec![(
            existing_pid,
            CanisterEntrySnapshot {
                role: role.clone(),
                parent_pid: Some(root_pid),
                module_hash: None,
                created_at: 1,
            },
        )],
    };

    let err = RegistryPolicy::can_register_role(&role, &snapshot, &single_canister_config())
        .expect_err("policy should reject duplicate singleton role");
    match err {
        RegistryPolicyError::RoleAlreadyRegistered { role: err_role, pid } => {
            assert_eq!(err_role, role);
            assert_eq!(pid, existing_pid);
        }
    }

    SubnetRegistryOps::register_root(root_pid);
    SubnetRegistryOps::register_unchecked(existing_pid, &role, root_pid, vec![])
        .expect("register first canister");
    let duplicate_pid = crate::p(3);
    SubnetRegistryOps::register_unchecked(duplicate_pid, &role, root_pid, vec![])
        .expect("ops should allow duplicate role when policy is bypassed");

    let duplicates = SubnetRegistryOps::snapshot()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();

    assert_eq!(duplicates, 2);
}
