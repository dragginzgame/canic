use crate::{
    cdk::candid::Principal,
    config::Config,
    ids::{CanisterRole, SubnetRole},
    ops::runtime::env::{self, EnvSnapshot},
};

pub fn init_sharding_test_config() {
    // Minimal config + env snapshot for sharding policy tests.
    let toml = r#"
        [subnets.prime.canisters.manager]
        cardinality = "single"
        initial_cycles = "5T"

        [subnets.prime.canisters.manager.sharding.pools.primary]
        canister_role = "shard"
        [subnets.prime.canisters.manager.sharding.pools.primary.policy]
        capacity = 1
        max_shards = 2

        [subnets.prime.canisters.shard]
        cardinality = "many"
        initial_cycles = "5T"
    "#;

    Config::init_from_toml(toml).expect("init sharding test config");

    // Single synthetic principal for root/subnet/parent roles in tests.
    let root_pid = Principal::from_slice(&[1; 29]);
    let snapshot = EnvSnapshot {
        canister_role: Some(CanisterRole::from("manager")),
        subnet_role: Some(SubnetRole::PRIME),
        root_pid: Some(root_pid),
        prime_root_pid: Some(root_pid),
        subnet_pid: Some(root_pid),
        parent_pid: Some(root_pid),
    };

    env::import(snapshot).expect("init sharding test env");
}
