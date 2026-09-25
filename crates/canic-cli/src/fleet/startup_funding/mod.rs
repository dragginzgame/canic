//! Module: canic_cli::fleet::startup_funding
//!
//! Responsibility: render the no-effect startup funding scenario beside generated desired state.
//! Does not own: forecasting policy, admission or durable funding authority.
//! Boundary: report assumptions and shortfalls without presenting them as approved spending.

use super::format_cycles;
use canic_host::fleet_ensure::view::startup_funding::{
    StartupChildLocalDemand, StartupCoordinatorUsage, StartupDemandUnavailable,
    StartupFundingForecast, StartupNativeBalance, StartupRootFunding, StartupUsageUnavailable,
};
use std::fmt::Write as _;

pub(super) fn render(forecast: &StartupFundingForecast) -> String {
    let mut text = String::from(
        "startup_funding_scenario: pool readiness floor; fresh child grant ledgers; no execution burn\n\
         startup_funding_authority: estimate only; not included in approved funding\n",
    );
    writeln!(
        text,
        "  coordinator_balance: {}",
        balance(forecast.coordinator_balance)
    )
    .unwrap();
    writeln!(
        text,
        "  coordinator_reserve: {}",
        format_cycles(forecast.coordinator_reserve_cycles)
    )
    .unwrap();
    writeln!(
        text,
        "  coordinator_spendable_before_grant_limits: {}",
        format_cycles(forecast.coordinator_spendable_cycles)
    )
    .unwrap();
    render_usage(&mut text, &forecast.coordinator_usage);
    for root in &forecast.roots {
        render_root(&mut text, root);
    }
    text
}

fn render_usage(text: &mut String, usage: &StartupCoordinatorUsage) {
    match usage {
        StartupCoordinatorUsage::Unavailable(reason) => {
            let reason = match reason {
                StartupUsageUnavailable::NotObserved => "not observed",
                StartupUsageUnavailable::NotWorkload => "asset is not an active Workload",
                StartupUsageUnavailable::ParentRelayRequired => {
                    "funding parent requires an update relay"
                }
                StartupUsageUnavailable::ParentNotObserved => "funding parent was not observed",
                StartupUsageUnavailable::SelectedBuildNotInstalled => {
                    "selected build not installed"
                }
                StartupUsageUnavailable::ObservationFailed => "protected query failed",
                StartupUsageUnavailable::AuthorityMismatch => "authority mismatch",
                StartupUsageUnavailable::PolicyTransition => {
                    "policy differs or rotation is in progress"
                }
                StartupUsageUnavailable::InvalidAccounting => "invalid accounting",
                StartupUsageUnavailable::InventoryIncomplete => "live inventory is incomplete",
            };
            writeln!(
                text,
                "  coordinator_grant_usage: unavailable ({reason}); no unused allowance assumed"
            )
            .unwrap();
        }
        StartupCoordinatorUsage::Observed(value) => {
            writeln!(
                text,
                "  coordinator_grant_usage: observed; generation {}; funding_enabled={}",
                value.policy_generation, value.funding_enabled
            )
            .unwrap();
            writeln!(
                text,
                "    automatic_grants: {}/{}; automatic_cycles: {}/{}; pending_roots: {}",
                value.automatic_grants,
                value.maximum_automatic_grants,
                value.automatic_cycles,
                value.maximum_automatic_cycles,
                value.pending_roots
            )
            .unwrap();
            writeln!(text, "    funding_window: start={}s; spent={} cycles; reserved={} cycles; remaining={} cycles", value.window_start_secs, value.spent_cycles, value.reserved_cycles, value.window_remaining_cycles).unwrap();
            writeln!(text, "    allowance_scope: current window only; not native balance or approval; Root limits and pending operations still apply").unwrap();
        }
    }
}

fn render_inventory(text: &mut String, root: &StartupRootFunding) {
    match root.inventory {
        Ok(coverage) => writeln!(
            text,
            "    live_inventory: complete; components={}; descendants={}",
            coverage.components, coverage.descendants
        ),
        Err(reason) => writeln!(text, "    live_inventory: unavailable ({reason:?})"),
    }
    .unwrap();
}

fn render_relay_quote(text: &mut String, root: &StartupRootFunding) {
    match &root.relay_quote {
        Ok(quote) => {
            writeln!(text, "    funding_observation_quote: attempts={}; proposed_burn_allowance={} cycles; per_attempt={} cycles", quote.requests.len(), quote.proposed_burn_allowance_cycles, quote.per_attempt_burn_allowance_cycles).unwrap();
            writeln!(text, "    observation_native_requirement: recovery_floor={}; required={}; observed={}; shortfall={} cycles", quote.minimum_recovery_cycles, quote.required_native_cycles, quote.observed_native_cycles, quote.native_shortfall_cycles).unwrap();
            writeln!(text, "    observation_quote_scope: one attempt per edge; excludes retries; requires separate review and durable spending authority").unwrap();
        }
        Err(reason) => writeln!(
            text,
            "    funding_observation_quote: unavailable ({reason:?})"
        )
        .unwrap(),
    }
}

fn render_recovery(text: &mut String, root: &StartupRootFunding) {
    match &root.recovery_demand {
        Ok(demand) => writeln!(text, "    live_recovery: root_grants={}; minimum_native={}; shortfall={}; uncovered={} cycles; exceeds_root_window={}", demand.root_grants_cycles, demand.minimum_native_cycles, demand.shortfall_cycles, demand.uncovered_cycles, demand.exceeds_root_window_budget),
        Err(reason) => writeln!(text, "    live_recovery: unavailable ({reason:?})"),
    }.unwrap();
}

fn render_root(text: &mut String, root: &StartupRootFunding) {
    writeln!(text, "  root: {}", root.root).unwrap();
    render_inventory(text, root);
    render_relay_quote(text, root);
    render_recovery(text, root);
    for child in &root.child_usage {
        if let Some(binding) = &child.binding {
            writeln!(
                text,
                "    child_funding_parent: {}; child={}; role={}; component_spec={}",
                binding.parent, child.child, binding.role, binding.component.component_spec
            )
            .unwrap();
        }
        match &child.usage {
            Ok(usage) => {
                writeln!(text, "    child_grant_usage: {} {}; accounted={} cycles; pending_operations={}; reserved_cycles={}",
                    child.name, child.child, usage.accounted_cycles, usage.pending_operations,
                    usage.reserved_cycles.map_or_else(|| "unknown".into(), |cycles| cycles.to_string())).unwrap();
            }
            Err(reason) => {
                writeln!(text, "    child_grant_usage: {} {}; unavailable ({reason:?}); no unused allowance assumed", child.name, child.child).unwrap();
            }
        }
        match &child.allowance {
            Ok(allowance) => {
                writeln!(text, "    child_grant_allowance: lifetime_limit={} cycles; remaining_after_charges={} cycles; cooldown_remaining={}s; next_request_policy_cap_cycles={}",
                    allowance.maximum_per_child_cycles, allowance.remaining_after_charges_cycles,
                    allowance.cooldown_remaining_secs,
                    allowance.next_request_policy_cap_cycles.map_or_else(|| "unknown".into(), |amount| amount.to_string())).unwrap();
            }
            Err(reason) => {
                writeln!(text, "    child_grant_allowance: unavailable ({reason:?}); no unused allowance assumed").unwrap();
            }
        }
        render_local_demand(text, &child.local_demand);
    }
    if !root.child_usage.is_empty() {
        writeln!(text, "    child_usage_scope: allocation-qualified Root-funded Workloads only; descendant usage requires parent relay; charged totals may include pending grants; recovery quote remains separate").unwrap();
        writeln!(text, "    child_allowance_scope: policy cap only; funding enablement, parent reserves and window eligibility still apply; not remaining demand or approved spending").unwrap();
        writeln!(text, "    child_local_demand_scope: own threshold only; excludes descendant transfers and burn; balance and ledger are separate observations; next request still requires parent admission; not a recovery quote").unwrap();
    }
    writeln!(text, "    native_balance: {}", balance(root.balance)).unwrap();
    writeln!(
        text,
        "    initial_child_grants: {}",
        format_cycles(root.child_grants_cycles)
    )
    .unwrap();
    writeln!(
        text,
        "    native_requirement_before_burn: {} ({} cycles)",
        format_cycles(root.minimum_native_cycles),
        root.minimum_native_cycles
    )
    .unwrap();
    writeln!(
        text,
        "    native_shortfall_before_burn: {} ({} cycles)",
        format_cycles(root.shortfall_cycles),
        root.shortfall_cycles
    )
    .unwrap();
    writeln!(
        text,
        "    grants_exceed_configured_window: {} ({} / {}s)",
        root.exceeds_window_budget,
        format_cycles(root.funding_budget.maximum_cycles.to_u128()),
        root.funding_budget.window_secs
    )
    .unwrap();
    for component in &root.components {
        writeln!(
            text,
            "    component: {} {}/{}",
            component.component_spec, component.deployment, component.ordinal
        )
        .unwrap();
        writeln!(
            text,
            "      descendant_grants: {} (included through parent demand)",
            format_cycles(component.descendant_grants_cycles)
        )
        .unwrap();
        writeln!(
            text,
            "      grants_exceed_configured_window: {} ({} / {}s)",
            component.exceeds_window_budget,
            format_cycles(component.funding_budget.maximum_cycles.to_u128()),
            component.funding_budget.window_secs
        )
        .unwrap();
        for role in &component.roles {
            writeln!(text, "      role: {} instances={} initial={} threshold={} grant_cap={} lifetime_limit={} grants_per_instance={} parent_funding_per_instance={} unfunded_per_instance_cycles={} cooldown={}s",
                    role.role, role.instances, format_cycles(role.initial_balance_cycles),
                    role.threshold_cycles.map_or_else(|| "disabled".to_owned(), format_cycles),
                    format_cycles(role.effective_grant_cycles), format_cycles(role.maximum_per_child_cycles),
                    role.grants_per_instance, format_cycles(role.parent_grants_per_instance_cycles),
                    role.unfunded_per_instance_cycles, role.cooldown_secs).unwrap();
        }
    }
}

fn render_local_demand(
    text: &mut String,
    demand: &Result<StartupChildLocalDemand, StartupDemandUnavailable>,
) {
    match demand {
        Ok(demand) => {
            writeln!(text, "    child_local_demand: balance={} cycles; threshold={}; shortfall={} cycles; beyond_lifetime_allowance={} cycles; next_request_policy_cycles={}",
                demand.observed_balance_cycles,
                demand.threshold_cycles.map_or_else(|| "disabled".into(), |amount| amount.to_string()),
                demand.shortfall_cycles, demand.shortfall_beyond_lifetime_allowance_cycles,
                demand.next_request_policy_cycles).unwrap();
        }
        Err(reason) => {
            writeln!(
                text,
                "    child_local_demand: unavailable ({reason:?}); no zero demand assumed"
            )
            .unwrap();
        }
    }
}

fn balance(value: StartupNativeBalance) -> String {
    match value {
        StartupNativeBalance::ConfiguredCreation(cycles) => {
            format!("{} (configured creation)", format_cycles(cycles))
        }
        StartupNativeBalance::Observed(cycles) => format!("{} (observed)", format_cycles(cycles)),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{cdk::types::Cycles, ids::CyclesFundingBudget};

    fn forecast() -> StartupFundingForecast {
        StartupFundingForecast {
            coordinator_usage: StartupCoordinatorUsage::Unavailable(
                StartupUsageUnavailable::NotObserved,
            ),
            coordinator_balance: StartupNativeBalance::Observed(0),
            coordinator_reserve_cycles: 1,
            coordinator_spendable_cycles: 0,
            roots: vec![StartupRootFunding {
                recovery_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
                relay_quote: Err(StartupDemandUnavailable::Usage(
                    StartupUsageUnavailable::NotObserved,
                )),
                inventory: Err(StartupUsageUnavailable::NotObserved),
                child_usage: Vec::new(),
                balance: StartupNativeBalance::ConfiguredCreation(10_000_000_000_000),
                child_grants_cycles: 0,
                components: vec![],
                exceeds_window_budget: false,
                funding_budget: CyclesFundingBudget {
                    window_secs: 60,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
                minimum_native_cycles: 10_000_000_000_001,
                request_threshold_cycles: 10_000_000_000_000,
                root: "root-0".to_owned(),
                shortfall_cycles: 1,
            }],
        }
    }

    fn child_binding() -> canic_host::fleet_ensure::view::startup_funding::StartupChildFundingBinding
    {
        use canic_core::ids::*;
        let principal = |byte| candid::Principal::from_slice(&[byte]);
        canic_host::fleet_ensure::view::startup_funding::StartupChildFundingBinding {
            release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [2; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([3; 32]),
            },
            component: ComponentBinding {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                                fleet_id: FleetId::from_generated_bytes([4; 32]),
                            },
                            app: AppId::from("test"),
                        },
                        coordinator_subnet: SubnetId::from_principal(principal(1)),
                        coordinator: principal(2),
                        recovery_controllers: Vec::new(),
                    },
                    epoch: 1,
                },
                component: ComponentInstanceId::from_generated_bytes([5; 32]),
                component_spec: "hubs".parse().unwrap(),
                spec_hash: [1; 32],
                role: "hub".into(),
                placement_subnet: SubnetId::from_principal(principal(1)),
                fleet_subnet_root: principal(3),
                canister_id: principal(4),
            },
            parent: principal(4),
            canister_id: principal(5),
            parent_role: Some("hub".into()),
            role: "shard".into(),
        }
    }

    #[test]
    fn startup_report_distinguishes_evidence_and_retains_sub_display_unit_shortfall() {
        let mut forecast = forecast();
        let report = render(&forecast);
        assert!(report.contains("coordinator_balance: 0.000B (observed)"));
        assert!(report.contains("native_balance: 10.000T (configured creation)"));
        assert!(report.contains("native_shortfall_before_burn: 0.000B (1 cycles)"));
        assert!(report.contains("startup_funding_authority: estimate only"));
        assert!(report.contains("coordinator_grant_usage: unavailable (not observed)"));
        forecast.coordinator_usage = StartupCoordinatorUsage::Observed(
            canic_host::fleet_ensure::view::startup_funding::StartupCoordinatorAccounting {
                policy_generation: 1,
                funding_enabled: false,
                window_start_secs: 60,
                spent_cycles: 30,
                reserved_cycles: 25,
                window_remaining_cycles: 45,
                automatic_grants: 2,
                maximum_automatic_grants: 4,
                automatic_cycles: 60,
                maximum_automatic_cycles: 200,
                pending_roots: 1,
            },
        );
        let report = render(&forecast);
        assert!(report.contains("funding_enabled=false"));
        assert!(
            report.contains("automatic_grants: 2/4; automatic_cycles: 60/200; pending_roots: 1")
        );
        assert!(report.contains("spent=30 cycles; reserved=25 cycles; remaining=45 cycles"));
        forecast.roots[0].child_usage.push(
            canic_host::fleet_ensure::view::startup_funding::StartupChildFundingUsage {
                observed_balance_cycles: Some(10),
                local_demand: Err(canic_host::fleet_ensure::view::startup_funding::StartupDemandUnavailable::PendingGrant),
                allowance: Ok(
                    canic_host::fleet_ensure::view::startup_funding::StartupChildFundingAllowance {
                        maximum_per_child_cycles: 200,
                        remaining_after_charges_cycles: 70,
                        cooldown_remaining_secs: 5,
                        next_request_policy_cap_cycles: None,
                    },
                ),
                binding: None,
                name: "pool-0".into(),
                child: "child".into(),
                usage: Ok(
                    canic_host::fleet_ensure::view::startup_funding::StartupChildAccounting {
                        observed_at_ns: 1_000_000_000,
                        accounted_cycles: 130,
                        last_accounted_at_secs: 1,
                        pending_operations: 1,
                        reserved_cycles: None,
                    },
                ),
            },
        );
        let report = render(&forecast);
        assert!(
            report.contains("accounted=130 cycles; pending_operations=1; reserved_cycles=unknown")
        );
        assert!(report.contains("allocation-qualified Root-funded Workloads only"));
        assert!(report.contains("child_local_demand: unavailable (PendingGrant)"));
        assert!(report.contains("remaining_after_charges=70 cycles; cooldown_remaining=5s; next_request_policy_cap_cycles=unknown"));
        forecast.roots[0].child_usage[0].binding = Some(child_binding());
        forecast.roots[0].child_usage[0].usage = Err(
            canic_host::fleet_ensure::view::startup_funding::StartupUsageUnavailable::ParentRelayRequired,
        );
        let report = render(&forecast);
        assert!(report.contains(&format!(
            "parent: {}; child=child; role=shard; component_spec=hubs",
            child_binding().parent
        )));
        assert!(report.contains("ParentRelayRequired"));
    }

    #[test]
    fn local_demand_report_preserves_exact_deficits() {
        let mut report = String::new();
        render_local_demand(
            &mut report,
            &Ok(StartupChildLocalDemand {
                observed_balance_cycles: 10,
                threshold_cycles: Some(20),
                shortfall_cycles: 11,
                shortfall_beyond_lifetime_allowance_cycles: 6,
                next_request_policy_cycles: 5,
            }),
        );
        assert!(report.contains("balance=10 cycles; threshold=20; shortfall=11 cycles; beyond_lifetime_allowance=6 cycles; next_request_policy_cycles=5"));
    }
}
