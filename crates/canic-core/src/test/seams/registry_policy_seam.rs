// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    InternalError,
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, DirectoryConfig, DirectoryPool,
        MetricsCanisterConfig, RandomnessConfig, ScalePool, ScalePoolPolicy, ScalingConfig,
        ShardingConfig, StandardsCanisterConfig,
    },
    domain::policy::topology::registry::{
        RegistryPolicy, RegistryPolicyError, RegistryRegistrationObservation,
    },
    domain::policy::topology::{RegistryPolicyInput, TopologyPolicyError, TopologyPolicyInput},
    dto::error::{Error, ErrorCode},
    ids::CanisterRole,
    ops::storage::registry::subnet::SubnetRegistryOps,
    test::seams::{lock, p},
};

fn root_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Root,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn singleton_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn singleton_scaling_parent_config() -> CanisterConfig {
    let mut scaling = ScalingConfig::default();
    scaling.pools.insert(
        "replicas".to_string(),
        ScalePool {
            canister_role: CanisterRole::new("replica_role"),
            policy: ScalePoolPolicy::default(),
        },
    );

    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: Some(scaling),
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn singleton_sharding_parent_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: Some(ShardingConfig::default()),
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

// Build a singleton parent that owns one keyed directory pool for instances.
fn singleton_directory_parent_config() -> CanisterConfig {
    let mut directory = DirectoryConfig::default();
    directory.pools.insert(
        "projects".to_string(),
        DirectoryPool {
            canister_role: CanisterRole::new("instance_child"),
            key_name: "project".to_string(),
        },
    );

    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: Some(directory),
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn replica_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Replica,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn shard_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Shard,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn instance_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Instance,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

#[test]
fn registry_kind_policy_blocks_but_ops_allows() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }

    let role = CanisterRole::new("seam_registry_singleton");
    let parent_role = CanisterRole::ROOT;
    let existing_pid = p(1);
    let root_pid = p(2);

    let data = RegistryPolicyInput {
        entries: vec![TopologyPolicyInput {
            pid: existing_pid,
            role: role.clone(),
            parent_pid: Some(root_pid),
            module_hash: None,
        }],
    };

    let err = RegistryPolicy::can_register_role(
        &role,
        root_pid,
        &data,
        &root_canister_config(),
        &parent_role,
        &root_canister_config(),
    )
    .expect_err("policy should reject duplicate singleton role");
    match &err {
        RegistryPolicyError::RoleAlreadyRegistered {
            role: err_role,
            pid,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(*pid, existing_pid);
        }
        RegistryPolicyError::SingletonAlreadyRegisteredUnderParent { .. }
        | RegistryPolicyError::ReplicaRequiresSingletonWithScaling { .. }
        | RegistryPolicyError::ShardRequiresSingletonWithSharding { .. }
        | RegistryPolicyError::InstanceRequiresSingletonWithDirectory { .. } => {
            panic!("expected root duplicate role error")
        }
    }

    let public = Error::from(InternalError::from(TopologyPolicyError::from(err)));
    assert_eq!(public.code, ErrorCode::PolicyRoleAlreadyRegistered);

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
fn registry_singleton_policy_blocks_under_parent() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::remove(&pid);
    }

    let role = CanisterRole::new("seam_registry_singleton_child");
    let parent_role = CanisterRole::new("singleton_parent");
    let parent_pid = p(4);
    let existing_pid = p(5);

    let data = RegistryPolicyInput {
        entries: vec![TopologyPolicyInput {
            pid: existing_pid,
            role: role.clone(),
            parent_pid: Some(parent_pid),
            module_hash: None,
        }],
    };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &singleton_canister_config(),
        &parent_role,
        &singleton_canister_config(),
    )
    .expect_err("policy should reject duplicate singleton role under parent");

    match &err {
        RegistryPolicyError::SingletonAlreadyRegisteredUnderParent {
            role: err_role,
            parent_pid: err_parent,
            pid,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(*err_parent, parent_pid);
            assert_eq!(*pid, existing_pid);
        }
        RegistryPolicyError::RoleAlreadyRegistered { .. }
        | RegistryPolicyError::ReplicaRequiresSingletonWithScaling { .. }
        | RegistryPolicyError::ShardRequiresSingletonWithSharding { .. }
        | RegistryPolicyError::InstanceRequiresSingletonWithDirectory { .. } => {
            panic!("expected duplicate singleton under parent error");
        }
    }

    let public = Error::from(InternalError::from(TopologyPolicyError::from(err)));
    assert_eq!(
        public.code,
        ErrorCode::PolicySingletonAlreadyRegisteredUnderParent
    );
    assert!(public.message.contains("singleton role"));
}

#[test]
fn registry_wasm_store_policy_allows_multiple_under_same_parent() {
    let role = CanisterRole::WASM_STORE;
    let parent_role = CanisterRole::ROOT;
    let parent_pid = p(6);
    let existing_pid = p(7);

    let data = RegistryPolicyInput {
        entries: vec![TopologyPolicyInput {
            pid: existing_pid,
            role: role.clone(),
            parent_pid: Some(parent_pid),
            module_hash: None,
        }],
    };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &singleton_canister_config(),
        &parent_role,
        &root_canister_config(),
    )
    .expect("wasm_store fleet role should allow multiple stores under the same root");
}

#[test]
fn instance_creation_requires_singleton_directory_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("plain_parent");
    let parent_pid = p(7);
    let data = RegistryPolicyInput { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &instance_canister_config(),
        &parent_role,
        &root_canister_config(),
    )
    .expect_err("policy should reject instance creation under non-singleton parent");

    match &err {
        RegistryPolicyError::InstanceRequiresSingletonWithDirectory {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected instance singleton-parent policy error"),
    }

    let public = Error::from(InternalError::from(TopologyPolicyError::from(err)));
    assert_eq!(
        public.code,
        ErrorCode::PolicyInstanceRequiresSingletonWithDirectory
    );
    assert!(
        public
            .message
            .contains("must be created by a singleton parent with directory config")
    );
}

#[test]
fn instance_creation_requires_directory_config_on_singleton_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("project_hub");
    let parent_pid = p(9);
    let data = RegistryPolicyInput { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &instance_canister_config(),
        &parent_role,
        &singleton_canister_config(),
    )
    .expect_err("policy should reject instance creation under singleton parent without directory");

    match &err {
        RegistryPolicyError::InstanceRequiresSingletonWithDirectory {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected instance singleton-directory policy error"),
    }
}

#[test]
fn instance_creation_succeeds_under_singleton_directory_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("project_hub");
    let parent_pid = p(10);
    let data = RegistryPolicyInput { entries: vec![] };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &instance_canister_config(),
        &parent_role,
        &singleton_directory_parent_config(),
    )
    .expect("instance should be allowed under singleton directory parent");
}

#[test]
fn replica_creation_succeeds_under_singleton_scaling_parent() {
    let role = CanisterRole::new("replica_child");
    let parent_role = CanisterRole::new("scale_hub");
    let parent_pid = p(8);
    let data = RegistryPolicyInput { entries: vec![] };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &replica_canister_config(),
        &parent_role,
        &singleton_scaling_parent_config(),
    )
    .expect("replica should be allowed under singleton scaling parent");
}

#[test]
fn shard_creation_succeeds_under_singleton_sharding_parent() {
    let role = CanisterRole::new("shard_child");
    let parent_role = CanisterRole::new("shard_hub");
    let parent_pid = p(9);
    let data = RegistryPolicyInput { entries: vec![] };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        &shard_canister_config(),
        &parent_role,
        &singleton_sharding_parent_config(),
    )
    .expect("shard should be allowed under singleton sharding parent");
}

#[test]
fn observed_registration_policy_matches_duplicate_singleton_decision() {
    let role = CanisterRole::new("observed_singleton_child");
    let parent_role = CanisterRole::new("singleton_parent");
    let parent_pid = p(11);
    let existing_pid = p(12);

    let err = RegistryPolicy::can_register_role_observed(
        &role,
        parent_pid,
        RegistryRegistrationObservation {
            existing_role_pid: Some(existing_pid),
            existing_singleton_under_parent_pid: Some(existing_pid),
        },
        &singleton_canister_config(),
        &parent_role,
        &singleton_canister_config(),
    )
    .expect_err("observed policy should reject duplicate singleton role under parent");

    match err {
        RegistryPolicyError::SingletonAlreadyRegisteredUnderParent {
            role: err_role,
            parent_pid: err_parent,
            pid,
        } => {
            assert_eq!(err_role, role);
            assert_eq!(err_parent, parent_pid);
            assert_eq!(pid, existing_pid);
        }
        other => panic!("unexpected observed policy error: {other}"),
    }
}

#[test]
fn observed_registration_policy_accepts_replica_without_registry_snapshot() {
    let role = CanisterRole::new("replica_child_observed");
    let parent_role = CanisterRole::new("scale_hub");

    RegistryPolicy::can_register_role_observed(
        &role,
        p(13),
        RegistryRegistrationObservation::default(),
        &replica_canister_config(),
        &parent_role,
        &singleton_scaling_parent_config(),
    )
    .expect("replica should be allowed from observed parent config without full registry input");
}
