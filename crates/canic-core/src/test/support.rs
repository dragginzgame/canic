use crate::{
    cdk::candid::Principal,
    cdk::types::Cycles,
    config::schema::{
        CanisterConfig, CanisterKind, RandomnessConfig, ShardPool, ShardPoolPolicy, ShardingConfig,
    },
    ids::{CanisterRole, SubnetRole},
    ops::runtime::env::EnvOps,
    storage::stable::env::EnvData,
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
                max_shards: 2,
            },
        },
    );

    let root_cfg = CanisterConfig {
        kind: CanisterKind::Root,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
    };

    let manager_cfg = CanisterConfig {
        kind: CanisterKind::Node,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: Some(sharding),
    };

    let shard_cfg = CanisterConfig {
        kind: CanisterKind::Shard,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
    };

    let _config = ConfigTestBuilder::new()
        .with_prime_canister(CanisterRole::ROOT, root_cfg)
        .with_prime_canister("manager", manager_cfg)
        .with_prime_canister("shard", shard_cfg)
        .install();

    // Single synthetic principal for root/subnet/parent roles in tests.
    let root_pid = Principal::from_slice(&[1; 29]);
    let snapshot = EnvData {
        canister_role: Some(CanisterRole::from("manager")),
        subnet_role: Some(SubnetRole::PRIME),
        root_pid: Some(root_pid),
        prime_root_pid: Some(root_pid),
        subnet_pid: Some(root_pid),
        parent_pid: Some(root_pid),
    };

    EnvOps::import(snapshot).expect("init sharding test env");
}
