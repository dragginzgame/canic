// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::candid::Principal,
    cdk::types::Cycles,
    config::schema::{
        CanisterConfig, CanisterKind, DelegatedAuthCanisterConfig, RandomnessConfig, ShardPool,
        ShardPoolPolicy, ShardingConfig, StandardsCanisterConfig,
    },
    ids::{CanisterRole, SubnetRole},
    ops::runtime::env::EnvOps,
    storage::stable::env::EnvRecord,
    test::config::ConfigTestBuilder,
};

pub fn init_sharding_test_config() {
    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "primary".to_string(),
        ShardPool {
            canister_role: CanisterRole::from("shard"),
            policy: ShardPoolPolicy {
                capacity: 1,
                initial_shards: 1,
                max_shards: 2,
            },
        },
    );

    let root_cfg = CanisterConfig {
        kind: CanisterKind::Root,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup_policy: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        delegated_auth: DelegatedAuthCanisterConfig::default(),
        standards: StandardsCanisterConfig::default(),
    };

    let manager_cfg = CanisterConfig {
        kind: CanisterKind::Singleton,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup_policy: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: Some(sharding),
        directory: None,
        delegated_auth: DelegatedAuthCanisterConfig::default(),
        standards: StandardsCanisterConfig::default(),
    };

    let shard_cfg = CanisterConfig {
        kind: CanisterKind::Shard,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup_policy: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        delegated_auth: DelegatedAuthCanisterConfig::default(),
        standards: StandardsCanisterConfig::default(),
    };

    let _config = ConfigTestBuilder::new()
        .with_prime_canister(CanisterRole::ROOT, root_cfg)
        .with_prime_canister("manager", manager_cfg)
        .with_prime_canister("shard", shard_cfg)
        .install();

    // Single synthetic principal for root/subnet/parent roles in tests.
    let root_pid = Principal::from_slice(&[1; 29]);
    import_test_env("manager", SubnetRole::PRIME, root_pid);
}

/// Imports a synthetic runtime env for unit tests.
pub fn import_test_env(
    canister_role: impl Into<CanisterRole>,
    subnet_role: impl Into<SubnetRole>,
    root_pid: Principal,
) {
    let snapshot = EnvRecord {
        canister_role: Some(canister_role.into()),
        subnet_role: Some(subnet_role.into()),
        root_pid: Some(root_pid),
        prime_root_pid: Some(root_pid),
        subnet_pid: Some(root_pid),
        parent_pid: Some(root_pid),
    };

    EnvOps::import(snapshot).expect("import test env");
}
