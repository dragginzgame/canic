// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::candid::Principal,
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, CyclesFundingPolicyConfig,
        DiagnosticsCanisterConfig, MetricsCanisterConfig, ShardPool, ShardPoolPolicy,
        ShardingConfig, StandardsCanisterConfig,
    },
    ids::{CanisterRole, CanonicalNetworkId, ComponentSpecId, FleetId, FleetKey},
    ops::runtime::env::EnvOps,
    storage::stable::env::{EnvData, EnvRecord},
    test::config::ConfigTestBuilder,
};

#[must_use]
pub fn fleet_key(byte: u8) -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([byte.saturating_add(1); 32]),
    }
}

/// Install the canonical sharding test configuration.
///
/// # Panics
///
/// Panics only if Canic's canonical `"default"` Component Spec identifier stops
/// satisfying `ComponentSpecId` admission.
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
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    };

    let manager_cfg = CanisterConfig {
        kind: CanisterKind::Service,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: Some(sharding),
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    };

    let shard_cfg = CanisterConfig {
        kind: CanisterKind::Shard,
        initial_cycles: Cycles::new(5_000_000_000_000),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    };

    let _config = ConfigTestBuilder::new()
        .with_default_canister(CanisterRole::ROOT, root_cfg)
        .with_default_canister("manager", manager_cfg)
        .with_default_canister("shard", shard_cfg)
        .install();

    // Single synthetic principal for root/subnet/parent roles in tests.
    let root_pid = Principal::from_slice(&[1; 29]);
    import_test_env(
        "manager",
        ComponentSpecId::try_from(String::from("default")).expect("default Component Spec ID"),
        root_pid,
    );
}

/// Imports a synthetic runtime env for unit tests.
///
/// # Panics
///
/// Panics if the synthetic environment snapshot fails runtime import.
pub fn import_test_env(
    canister_role: impl Into<CanisterRole>,
    component_spec: impl Into<ComponentSpecId>,
    root_pid: Principal,
) {
    let snapshot = EnvRecord {
        managed_binding: None,
        canister_role: Some(canister_role.into()),
        component_spec: Some(component_spec.into()),
        root_pid: Some(root_pid),
        fleet_subnet_root_pid: Some(root_pid),
        subnet_pid: Some(root_pid),
        parent_pid: Some(root_pid),
    };

    EnvOps::import(EnvData { record: snapshot }).expect("import test env");
}
