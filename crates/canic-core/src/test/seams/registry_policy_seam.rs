use crate::{
    cdk::types::Cycles,
    config::schema::{CanisterConfig, CanisterKind, RandomnessConfig},
    domain::policy::topology::registry::{RegistryPolicy, RegistryPolicyError},
    ids::CanisterRole,
    ops::{
        storage::CanisterRecord,
        storage::registry::subnet::{SubnetRegistryData, SubnetRegistryOps},
    },
    test::seams::{lock, p},
};

fn single_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Root,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
    }
}

fn node_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Node,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
    }
}

#[test]
fn registry_kind_policy_blocks_but_ops_allows() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }

    let role = CanisterRole::new("seam_registry_singleton");
    let existing_pid = p(1);
    let root_pid = p(2);

    let data = SubnetRegistryData {
        entries: vec![(
            existing_pid,
            CanisterRecord {
                role: role.clone(),
                parent_pid: Some(root_pid),
                module_hash: None,
                created_at: 1,
            },
        )],
    };

    let err = RegistryPolicy::can_register_role(&role, root_pid, &data, &single_canister_config())
        .expect_err("policy should reject duplicate singleton role");
    match err {
        RegistryPolicyError::RoleAlreadyRegistered {
            role: err_role,
            pid,
        } => {
            assert_eq!(err_role, role);
            assert_eq!(pid, existing_pid);
        }
        RegistryPolicyError::RoleAlreadyRegisteredUnderParent { .. } => todo!(),
    }

    let created_at = 1;
    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(existing_pid, &role, root_pid, vec![], created_at)
        .expect("register first canister");
    let duplicate_pid = p(3);
    SubnetRegistryOps::register_unchecked(duplicate_pid, &role, root_pid, vec![], created_at)
        .expect("ops should allow duplicate role when policy is bypassed");

    let duplicates = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();

    assert_eq!(duplicates, 2);
}

#[test]
fn registry_node_policy_blocks_under_parent() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }

    let role = CanisterRole::new("seam_registry_node");
    let parent_pid = p(4);
    let existing_pid = p(5);

    let data = SubnetRegistryData {
        entries: vec![(
            existing_pid,
            CanisterRecord {
                role: role.clone(),
                parent_pid: Some(parent_pid),
                module_hash: None,
                created_at: 1,
            },
        )],
    };

    let err = RegistryPolicy::can_register_role(&role, parent_pid, &data, &node_canister_config())
        .expect_err("policy should reject duplicate node role under parent");

    match err {
        RegistryPolicyError::RoleAlreadyRegisteredUnderParent {
            role: err_role,
            parent_pid: err_parent,
            pid,
        } => {
            assert_eq!(err_role, role);
            assert_eq!(err_parent, parent_pid);
            assert_eq!(pid, existing_pid);
        }
        RegistryPolicyError::RoleAlreadyRegistered { .. } => {
            panic!("expected duplicate under parent error");
        }
    }
}
