// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::candid::Principal,
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, CyclesFundingPolicyConfig,
        DiagnosticsCanisterConfig, MetricsCanisterConfig, ShardPool, ShardPoolPolicy,
        ShardingConfig, StandardsCanisterConfig,
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
        ComponentSpecId, CyclesFundingBudget, FleetAdmissionPolicy, FleetAdmissionProjection,
        FleetBinding, FleetCoordinatorBinding, FleetFundingProfile, FleetId, FleetKey,
        FleetRegistryAuthority, FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy,
        ManagedCanisterBinding, SubnetId,
    },
    ops::{
        fleet_admission_policy::{
            bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
        },
        runtime::env::EnvOps,
    },
    storage::stable::env::{EnvData, EnvRecord},
    test::config::ConfigTestBuilder,
    workflow::fleet_admission_projection::compile_fleet_admission_projection,
};

#[must_use]
pub fn fleet_key(byte: u8) -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([byte.saturating_add(1); 32]),
    }
}

/// Return one valid protected root-funding authority for unrelated unit fixtures.
#[must_use]
pub fn fleet_subnet_root_funding_authority() -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        root_funding: FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            request_threshold: Cycles::new(10_000_000_000_000),
            target_balance: Cycles::new(30_000_000_000_000),
            cooldown_secs: 30 * 24 * 60 * 60,
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(30_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
        },
        icp_refill: None,
    }
}

/// Return one valid generation-one Fleet admission policy for unrelated fixtures.
///
/// # Panics
///
/// Panics only if the fixed canonical test Principal stops satisfying the
/// generation-one Fleet admission invariants.
#[must_use]
pub fn fleet_admission_policy(fleet: FleetBinding) -> FleetAdmissionPolicy {
    let template =
        compile_fleet_admission_policy_template(vec![Principal::from_slice(&[1; 29])], Vec::new())
            .expect("test Fleet admission template");
    bind_initial_fleet_admission_policy(fleet, &template).expect("test Fleet admission policy")
}

/// Return one exact managed Component binding for admission projection tests.
///
/// # Panics
///
/// Panics only if the fixed `"default"` Component Spec identifier stops
/// satisfying the maintained identifier contract.
#[must_use]
pub fn managed_component_binding() -> ManagedCanisterBinding {
    let fleet = FleetBinding {
        fleet: fleet_key(7),
        app: AppId::from("test"),
    };
    let placement_subnet = SubnetId::from_principal(Principal::from_slice(&[8; 29]));
    ManagedCanisterBinding::Component(ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet,
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[9; 29])),
                coordinator: Principal::from_slice(&[10; 29]),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([11; 32]),
        component_spec: ComponentSpecId::try_from(String::from("default"))
            .expect("default Component Spec ID"),
        spec_hash: [12; 32],
        role: CanisterRole::from("app"),
        placement_subnet,
        fleet_subnet_root: Principal::from_slice(&[13; 29]),
        canister_id: Principal::from_slice(&[14; 29]),
    })
}

/// Return one canonical local projection for the exact supplied test target.
///
/// # Panics
///
/// Panics only if the supplied target or fixed Fleet policy violates the
/// maintained projection contract.
#[must_use]
pub fn fleet_admission_projection(target: ManagedCanisterBinding) -> FleetAdmissionProjection {
    let fleet = match &target {
        ManagedCanisterBinding::Component(binding) => binding.authority.binding.fleet.clone(),
        ManagedCanisterBinding::ComponentChild(binding) => {
            binding.component.authority.binding.fleet.clone()
        }
    };
    compile_fleet_admission_projection(&fleet_admission_policy(fleet), target)
        .expect("test Fleet admission projection")
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
