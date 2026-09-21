use super::*;
use crate::fleet_ensure::{
    policy::startup_funding::{funding_binding, hub_config},
    view::startup_funding::{
        StartupChildFundingBinding, StartupChildFundingUsage, StartupInventoryCoverage,
    },
};
use candid::Principal;
use canic_core::control_plane_support::config::ComponentDeploymentConfiguration;

fn requests() -> Vec<StartupChildFundingBinding> {
    let config = ComponentDeploymentConfiguration::compile(&hub_config()).unwrap();
    let top = funding_binding(&config.component_topology.component_specs[0]);
    [7, 8]
        .into_iter()
        .map(|id| {
            let mut child = top.clone();
            child.parent = top.canister_id;
            child.parent_role = Some(top.role.clone());
            child.canister_id = Principal::from_slice(&[id]);
            child.role = "shard".into();
            child
        })
        .collect()
}

#[test]
fn relay_quote_preserves_native_recovery_before_observation_allowance() {
    for (balance, shortfall) in [(0, 120), (99, 21), (100, 20), (119, 1), (120, 0), (121, 0)] {
        let quote =
            calculate(requests(), StartupNativeBalance::Observed(balance), 100, 10).unwrap();
        assert_eq!(quote.requests, requests());
        assert_eq!(quote.proposed_burn_allowance_cycles, 20);
        assert_eq!(quote.required_native_cycles, 120);
        assert_eq!(quote.minimum_recovery_cycles, 100);
        assert_eq!(quote.native_shortfall_cycles, shortfall);
    }
}

#[test]
fn relay_quote_rejects_overflow_and_configured_balances() {
    for (minimum, attempt) in [(0, u128::MAX), (u128::MAX, 1)] {
        assert_eq!(
            calculate(
                requests(),
                StartupNativeBalance::Observed(u128::MAX),
                minimum,
                attempt
            ),
            Err(StartupDemandUnavailable::ArithmeticOverflow)
        );
    }
    assert_eq!(
        calculate(
            requests(),
            StartupNativeBalance::ConfiguredCreation(120),
            100,
            10
        ),
        Err(StartupDemandUnavailable::BalanceNotObserved)
    );
    let empty = calculate(Vec::new(), StartupNativeBalance::Observed(100), 100, 0).unwrap();
    assert_eq!(empty.proposed_burn_allowance_cycles, 0);
    assert_eq!(empty.required_native_cycles, 100);
}

/// Exercise exact selection and membership using the generated-estate fixture's real authority.
pub(in crate::fleet_ensure) fn qualify(
    desired: &DesiredFleet,
    root_name: &str,
    top: &StartupChildFundingBinding,
) {
    let mut desired = desired.clone();
    let configuration = ComponentDeploymentConfiguration::compile(&hub_config()).unwrap();
    let spec = &configuration.component_topology.component_specs[0];
    let mut top = top.clone();
    top.role = spec.component_role.clone();
    top.component.role = spec.component_role.clone();
    top.component.component_spec = spec.component_spec.clone();
    top.component.spec_hash = spec.spec_hash;
    let bootstrap = desired.bootstrap.as_mut().unwrap();
    bootstrap.roots[0].component_admissions = vec![canic_core::ids::ComponentSpecAdmission {
        component_spec: spec.component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances: 1,
    }];
    bootstrap.roots[0].component_topology_digest =
        configuration.component_topology.digest().unwrap();
    bootstrap.component_deployment_configuration = configuration;
    qualify_members(&desired, root_name, &top);
}

fn qualify_members(desired: &DesiredFleet, root_name: &str, top: &StartupChildFundingBinding) {
    let mut child = top.clone();
    child.canister_id = Principal::from_slice(&[97; 29]);
    child.parent = top.canister_id;
    child.parent_role = Some(top.role.clone());
    child.role = "shard".into();
    let mut sibling = child.clone();
    sibling.canister_id = Principal::from_slice(&[98; 29]);
    let mut root = StartupRootFunding {
        recovery_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
        relay_quote: Err(StartupDemandUnavailable::BalanceNotObserved),
        inventory: Ok(StartupInventoryCoverage {
            components: 1,
            descendants: 2,
        }),
        child_usage: [&sibling, top, &child]
            .into_iter()
            .map(|binding| StartupChildFundingUsage {
                allowance: Err(StartupUsageUnavailable::NotObserved),
                observed_balance_cycles: None,
                local_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
                binding: Some(binding.clone()),
                name: binding.canister_id.to_text(),
                child: binding.canister_id.to_text(),
                usage: Err(StartupUsageUnavailable::ParentRelayRequired),
            })
            .collect(),
        balance: StartupNativeBalance::Observed(0),
        child_grants_cycles: 0,
        components: Vec::new(),
        funding_budget: desired.bootstrap.as_ref().unwrap().roots[0]
            .limits
            .cycles_funding
            .clone(),
        exceeds_window_budget: false,
        minimum_native_cycles: 0,
        request_threshold_cycles: 0,
        root: root_name.into(),
        shortfall_cycles: 0,
    };
    crate::fleet_ensure::workflow::funding_observation::qualify(desired, &root);
    crate::fleet_ensure::policy::startup_funding::recursive::qualify(desired, &root);
    let quote = project(desired, &root).unwrap();
    let mut expected = vec![top.clone(), child, sibling];
    expected.sort_by_key(|binding| (binding.parent, binding.canister_id));
    assert_eq!(quote.requests, expected);
    let bounds = cycle_bounds(desired).unwrap();
    assert_eq!(
        quote.proposed_burn_allowance_cycles,
        6 * (bounds.observation_burn + bounds.update_burn)
    );
    assert_eq!(quote.native_shortfall_cycles, quote.required_native_cycles);
    root.child_usage.reverse();
    assert_eq!(project(desired, &root), Ok(quote));
    for field in ["observation", "update"] {
        let mut invalid = desired.clone();
        match field {
            "observation" => invalid.maximum_observation_burn_cycles = "0".into(),
            _ => invalid.maximum_update_burn_cycles = "0".into(),
        }
        assert_eq!(
            project(&invalid, &root),
            Err(StartupDemandUnavailable::InvalidObservationBounds)
        );
    }
    root.inventory = Err(StartupUsageUnavailable::PolicyTransition);
    assert_eq!(
        project(desired, &root),
        Err(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::PolicyTransition
        ))
    );
    root.inventory = Ok(StartupInventoryCoverage {
        components: 1,
        descendants: 3,
    });
    assert_eq!(
        project(desired, &root),
        Err(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::InventoryIncomplete
        ))
    );
    root.inventory = Ok(StartupInventoryCoverage {
        components: 1,
        descendants: 2,
    });
    root.child_usage
        .retain(|entry| entry.binding.as_ref() != Some(top));
    assert_eq!(
        project(desired, &root),
        Err(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::ParentNotObserved
        ))
    );
}
