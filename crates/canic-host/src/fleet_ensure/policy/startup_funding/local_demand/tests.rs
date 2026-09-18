use super::*;
use crate::fleet_ensure::policy::startup_funding::tests::hub_config;
use canic_core::{
    cdk::types::Cycles, control_plane_support::config::ComponentDeploymentConfiguration,
};

const T: u128 = 1_000_000_000_000;

fn config() -> ConfigModel {
    let mut config = hub_config();
    let policy = &mut config
        .component_specs
        .get_mut("hubs")
        .unwrap()
        .cycles_funding;
    policy.max_per_request = Cycles::new(30 * T);
    policy.max_per_child = Cycles::new(100 * T);
    policy.cooldown_secs = 20;
    config
}

fn usage() -> StartupChildAccounting {
    StartupChildAccounting {
        observed_at_ns: 70_000_000_000,
        accounted_cycles: 95 * T,
        last_accounted_at_secs: 50,
        pending_operations: 0,
        reserved_cycles: Some(0),
    }
}

fn binding(config: &ConfigModel) -> (ComponentTopology, StartupChildFundingBinding) {
    let topology = ComponentDeploymentConfiguration::compile(config)
        .unwrap()
        .component_topology;
    let spec = &topology.component_specs[0];
    let binding = super::super::tests::funding_binding(spec);
    (topology, binding)
}

fn demand(
    config: &ConfigModel,
    usage: &StartupChildAccounting,
    balance: Option<u128>,
) -> Result<StartupChildLocalDemand, StartupDemandUnavailable> {
    let (topology, binding) = binding(config);
    project(
        config,
        &topology,
        binding.release_set.release_build_id,
        &binding,
        usage,
        balance,
    )
}

#[test]
fn live_balance_and_charges_replace_fresh_ledger_assumptions() {
    let config = config();
    let result = demand(&config, &usage(), Some(4 * T)).unwrap();
    assert_eq!(result.shortfall_cycles, 6 * T + 1);
    assert_eq!(result.shortfall_beyond_lifetime_allowance_cycles, T + 1);
    assert_eq!(result.next_request_policy_cycles, 5 * T);
    // Existing descendants do not get silently folded into the local-only figure.
    assert_eq!(result.observed_balance_cycles, 4 * T);
    let equality = demand(&config, &usage(), Some(10 * T)).unwrap();
    assert_eq!(equality.shortfall_cycles, 1);
    assert_eq!(equality.next_request_policy_cycles, 5 * T);
    let above = demand(&config, &usage(), Some(10 * T + 1)).unwrap();
    assert_eq!(above.shortfall_cycles, 0);
    assert_eq!(above.next_request_policy_cycles, 0);
}

#[test]
fn cooldown_and_exhaustion_do_not_hide_demand() {
    let config = config();
    let mut usage = usage();
    usage.observed_at_ns -= 1_000_000_000;
    let waiting = demand(&config, &usage, Some(0)).unwrap();
    assert_eq!(waiting.shortfall_cycles, 10 * T + 1);
    assert_eq!(waiting.next_request_policy_cycles, 0);
    usage.observed_at_ns += 1_000_000_000;
    usage.accounted_cycles = 99 * T;
    assert_eq!(
        demand(&config, &usage, Some(0))
            .unwrap()
            .next_request_policy_cycles,
        T
    );
    usage.accounted_cycles = 100 * T;
    let exhausted = demand(&config, &usage, Some(0)).unwrap();
    assert_eq!(
        exhausted.shortfall_beyond_lifetime_allowance_cycles,
        exhausted.shortfall_cycles
    );
    assert_eq!(exhausted.next_request_policy_cycles, 0);
}

#[test]
fn missing_or_pending_evidence_is_not_zero_demand() {
    let config = config();
    let mut usage = usage();
    assert_eq!(
        demand(&config, &usage, None),
        Err(StartupDemandUnavailable::BalanceNotObserved)
    );
    usage.pending_operations = 1;
    for reserved in [Some(0), Some(5 * T), None] {
        usage.reserved_cycles = reserved;
        assert_eq!(
            demand(&config, &usage, Some(20 * T)),
            Err(StartupDemandUnavailable::PendingGrant)
        );
    }
    usage.pending_operations = 0;
    assert_eq!(
        demand(&config, &usage, Some(0)),
        Err(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::InvalidAccounting
        ))
    );
}

#[test]
fn local_demand_requires_exact_selected_policy_even_when_balance_is_high() {
    let config = config();
    let (topology, mut binding) = binding(&config);
    let release = binding.release_set.release_build_id;
    binding.component.spec_hash = [99; 32];
    assert_eq!(
        project(
            &config,
            &topology,
            release,
            &binding,
            &usage(),
            Some(u128::MAX)
        ),
        Err(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::PolicyTransition
        ))
    );
}

#[test]
fn disabled_topup_is_explicit_and_threshold_overflow_is_rejected() {
    let mut config = config();
    config.component_specs.get_mut("hubs").unwrap().topup = None;
    let disabled = demand(&config, &usage(), Some(0)).unwrap();
    assert_eq!(disabled.threshold_cycles, None);
    assert_eq!(disabled.next_request_policy_cycles, 0);
    config.component_specs.get_mut("hubs").unwrap().topup =
        hub_config().component_specs["hubs"].topup.clone();
    config
        .component_specs
        .get_mut("hubs")
        .unwrap()
        .topup
        .as_mut()
        .unwrap()
        .threshold = Cycles::new(u128::MAX);
    assert_eq!(
        demand(&config, &usage(), Some(0)),
        Err(StartupDemandUnavailable::ArithmeticOverflow)
    );
}
