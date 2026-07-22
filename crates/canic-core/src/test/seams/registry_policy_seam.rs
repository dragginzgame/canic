// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    InternalError,
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, CyclesFundingPolicyConfig,
        DiagnosticsCanisterConfig, DirectoryConfig, DirectoryPool, MetricsCanisterConfig,
        ScalePool, ScalePoolPolicy, ScalingConfig, ShardingConfig, StandardsCanisterConfig,
    },
    domain::policy::pure::topology::TopologyPolicyError,
    domain::policy::pure::topology::registry::{
        RegistryCanisterKind, RegistryCanisterShape, RegistryPolicy, RegistryPolicyError,
        RegistryRegistrationObservation,
    },
    dto::error::{Error, ErrorCode},
    ids::CanisterRole,
    model::topology::{TopologyEntry, TopologyRegistry},
    ops::storage::registry::subnet::SubnetRegistryOps,
    test::seams::{lock, p},
};

fn root_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Root,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn service_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Service,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn singleton_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
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
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: Some(scaling),
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn service_scaling_parent_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Service,
        ..singleton_scaling_parent_config()
    }
}

fn singleton_sharding_parent_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: Some(ShardingConfig::default()),
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn service_sharding_parent_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Service,
        ..singleton_sharding_parent_config()
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
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: Some(directory),
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn service_directory_parent_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Service,
        ..singleton_directory_parent_config()
    }
}

fn replica_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Replica,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn shard_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Shard,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn instance_canister_config() -> CanisterConfig {
    CanisterConfig {
        kind: CanisterKind::Instance,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn registry_shape(cfg: &CanisterConfig) -> RegistryCanisterShape {
    RegistryCanisterShape {
        kind: registry_kind(cfg.kind),
        has_scaling: cfg.scaling.is_some(),
        has_sharding: cfg.sharding.is_some(),
        has_directory: cfg.directory.is_some(),
    }
}

const fn registry_kind(kind: CanisterKind) -> RegistryCanisterKind {
    match kind {
        CanisterKind::Root => RegistryCanisterKind::Root,
        CanisterKind::Service => RegistryCanisterKind::Service,
        CanisterKind::Singleton => RegistryCanisterKind::Singleton,
        CanisterKind::Replica => RegistryCanisterKind::Replica,
        CanisterKind::Shard => RegistryCanisterKind::Shard,
        CanisterKind::Instance => RegistryCanisterKind::Instance,
    }
}

#[test]
fn registry_kind_policy_blocks_but_ops_allows() {
    let _guard = lock();

    for entry in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&entry.pid);
    }

    let role = CanisterRole::new("seam_registry_singleton");
    let parent_role = CanisterRole::ROOT;
    let existing_pid = p(1);
    let root_pid = p(2);

    let data = TopologyRegistry {
        entries: vec![TopologyEntry {
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
        registry_shape(&root_canister_config()),
        &parent_role,
        registry_shape(&root_canister_config()),
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
        | RegistryPolicyError::ServiceRequiresRootParent { .. }
        | RegistryPolicyError::ReplicaRequiresServiceWithScaling { .. }
        | RegistryPolicyError::ShardRequiresServiceWithSharding { .. }
        | RegistryPolicyError::InstanceRequiresServiceWithDirectory { .. } => {
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
        .filter(|entry| entry.record.role == role)
        .count();

    assert_eq!(duplicates, 2);
}

#[test]
fn registry_service_policy_blocks_duplicate_role() {
    let role = CanisterRole::new("project_hub");
    let parent_role = CanisterRole::ROOT;
    let existing_pid = p(11);
    let root_pid = p(12);

    let data = TopologyRegistry {
        entries: vec![TopologyEntry {
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
        registry_shape(&service_canister_config()),
        &parent_role,
        registry_shape(&root_canister_config()),
    )
    .expect_err("policy should reject duplicate service role");

    match &err {
        RegistryPolicyError::RoleAlreadyRegistered {
            role: err_role,
            pid,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(*pid, existing_pid);
        }
        RegistryPolicyError::SingletonAlreadyRegisteredUnderParent { .. }
        | RegistryPolicyError::ServiceRequiresRootParent { .. }
        | RegistryPolicyError::ReplicaRequiresServiceWithScaling { .. }
        | RegistryPolicyError::ShardRequiresServiceWithSharding { .. }
        | RegistryPolicyError::InstanceRequiresServiceWithDirectory { .. } => {
            panic!("expected service duplicate role error")
        }
    }
}

#[test]
fn registry_service_policy_requires_root_parent() {
    let role = CanisterRole::new("project_hub");
    let parent_role = CanisterRole::new("project_instance");
    let parent_pid = p(13);

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &TopologyRegistry { entries: vec![] },
        registry_shape(&service_canister_config()),
        &parent_role,
        registry_shape(&singleton_canister_config()),
    )
    .expect_err("service roles should be rejected under non-root parents");

    match err {
        RegistryPolicyError::ServiceRequiresRootParent {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, role);
            assert_eq!(err_parent_role, parent_role);
        }
        other => panic!("unexpected service parent policy error: {other}"),
    }
}

#[test]
fn registry_singleton_policy_blocks_under_parent() {
    let _guard = lock();

    for entry in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&entry.pid);
    }

    let role = CanisterRole::new("seam_registry_singleton_child");
    let parent_role = CanisterRole::new("singleton_parent");
    let parent_pid = p(4);
    let existing_pid = p(5);

    let data = TopologyRegistry {
        entries: vec![TopologyEntry {
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
        registry_shape(&singleton_canister_config()),
        &parent_role,
        registry_shape(&singleton_canister_config()),
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
        | RegistryPolicyError::ServiceRequiresRootParent { .. }
        | RegistryPolicyError::ReplicaRequiresServiceWithScaling { .. }
        | RegistryPolicyError::ShardRequiresServiceWithSharding { .. }
        | RegistryPolicyError::InstanceRequiresServiceWithDirectory { .. } => {
            panic!("expected duplicate singleton under parent error");
        }
    }

    let public = Error::from(InternalError::from(TopologyPolicyError::from(err)));
    assert_eq!(
        public.code,
        ErrorCode::PolicySingletonAlreadyRegisteredUnderParent
    );
}

#[test]
fn registry_wasm_store_policy_allows_multiple_under_same_parent() {
    let role = CanisterRole::WASM_STORE;
    let parent_role = CanisterRole::ROOT;
    let parent_pid = p(6);
    let existing_pid = p(7);

    let data = TopologyRegistry {
        entries: vec![TopologyEntry {
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
        registry_shape(&singleton_canister_config()),
        &parent_role,
        registry_shape(&root_canister_config()),
    )
    .expect("wasm_store fleet role should allow multiple stores under the same root");
}

#[test]
fn instance_creation_requires_service_directory_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("plain_parent");
    let parent_pid = p(7);
    let data = TopologyRegistry { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&instance_canister_config()),
        &parent_role,
        registry_shape(&root_canister_config()),
    )
    .expect_err("policy should reject instance creation under non-service parent");

    match &err {
        RegistryPolicyError::InstanceRequiresServiceWithDirectory {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected instance service-parent policy error"),
    }

    let public = Error::from(InternalError::from(TopologyPolicyError::from(err)));
    assert_eq!(
        public.code,
        ErrorCode::PolicyInstanceRequiresServiceWithDirectory
    );
}

#[test]
fn instance_creation_requires_directory_config_on_service_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("project_hub");
    let parent_pid = p(9);
    let data = TopologyRegistry { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&instance_canister_config()),
        &parent_role,
        registry_shape(&service_canister_config()),
    )
    .expect_err("policy should reject instance creation under service parent without directory");

    match &err {
        RegistryPolicyError::InstanceRequiresServiceWithDirectory {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected instance service-directory policy error"),
    }
}

#[test]
fn instance_creation_rejects_singleton_directory_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("project_hub");
    let parent_pid = p(10);
    let data = TopologyRegistry { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&instance_canister_config()),
        &parent_role,
        registry_shape(&singleton_directory_parent_config()),
    )
    .expect_err("singleton directory parents should not create instances");

    match &err {
        RegistryPolicyError::InstanceRequiresServiceWithDirectory {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected instance service-directory policy error"),
    }
}

#[test]
fn instance_creation_succeeds_under_service_directory_parent() {
    let role = CanisterRole::new("instance_child");
    let parent_role = CanisterRole::new("project_hub");
    let parent_pid = p(10);
    let data = TopologyRegistry { entries: vec![] };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&instance_canister_config()),
        &parent_role,
        registry_shape(&service_directory_parent_config()),
    )
    .expect("instance should be allowed under service directory parent");
}

#[test]
fn replica_creation_rejects_singleton_scaling_parent() {
    let role = CanisterRole::new("replica_child");
    let parent_role = CanisterRole::new("scale_hub");
    let parent_pid = p(8);
    let data = TopologyRegistry { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&replica_canister_config()),
        &parent_role,
        registry_shape(&singleton_scaling_parent_config()),
    )
    .expect_err("singleton scaling parents should not create replicas");

    match &err {
        RegistryPolicyError::ReplicaRequiresServiceWithScaling {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected replica service-scaling policy error"),
    }
}

#[test]
fn replica_creation_succeeds_under_service_scaling_parent() {
    let role = CanisterRole::new("replica_child");
    let parent_role = CanisterRole::new("scale_hub");
    let parent_pid = p(8);
    let data = TopologyRegistry { entries: vec![] };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&replica_canister_config()),
        &parent_role,
        registry_shape(&service_scaling_parent_config()),
    )
    .expect("replica should be allowed under service scaling parent");
}

#[test]
fn shard_creation_rejects_singleton_sharding_parent() {
    let role = CanisterRole::new("shard_child");
    let parent_role = CanisterRole::new("shard_hub");
    let parent_pid = p(9);
    let data = TopologyRegistry { entries: vec![] };

    let err = RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&shard_canister_config()),
        &parent_role,
        registry_shape(&singleton_sharding_parent_config()),
    )
    .expect_err("singleton sharding parents should not create shards");

    match &err {
        RegistryPolicyError::ShardRequiresServiceWithSharding {
            role: err_role,
            parent_role: err_parent_role,
        } => {
            assert_eq!(err_role, &role);
            assert_eq!(err_parent_role, &parent_role);
        }
        _ => panic!("expected shard service-sharding policy error"),
    }
}

#[test]
fn shard_creation_succeeds_under_service_sharding_parent() {
    let role = CanisterRole::new("shard_child");
    let parent_role = CanisterRole::new("shard_hub");
    let parent_pid = p(9);
    let data = TopologyRegistry { entries: vec![] };

    RegistryPolicy::can_register_role(
        &role,
        parent_pid,
        &data,
        registry_shape(&shard_canister_config()),
        &parent_role,
        registry_shape(&service_sharding_parent_config()),
    )
    .expect("shard should be allowed under service sharding parent");
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
        registry_shape(&singleton_canister_config()),
        &parent_role,
        registry_shape(&singleton_canister_config()),
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
        registry_shape(&replica_canister_config()),
        &parent_role,
        registry_shape(&service_scaling_parent_config()),
    )
    .expect("replica should be allowed from observed parent config without full registry input");
}
