//! Module: domain::policy::pure::fleet_funding
//!
//! Responsibility: decide one authenticated Coordinator-to-Root grant from immutable facts.
//! Does not own: DTOs, caller lookup, clocks, balances, persistence, serialization, or calls.
//! Boundary: Coordinator ops supplies already-resolved authority and commits the decision.

use crate::ids::{FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingPolicy};

/// Current usage in one exact epoch-anchored accounting window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FleetFundingWindowSnapshot {
    pub window_start_secs: u64,
    pub spent_cycles: u128,
    pub reserved_cycles: u128,
}

/// Non-renewing successful automatic usage for one installed policy generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FleetFundingAutomaticUsageSnapshot {
    pub successful_grants: u32,
    pub granted_cycles: u128,
}

/// Runtime availability facts for one fresh grant request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FleetRootGrantAvailability {
    pub funding_enabled: bool,
    pub root_is_eligible: bool,
}

/// Exact protected-authority comparisons for one fresh grant request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FleetRootGrantAuthorityMatch {
    pub registry_matches: bool,
    pub policy_matches: bool,
}

/// Immutable facts used to decide one fresh grant request.
pub struct FleetRootGrantDecisionInput<'a> {
    pub availability: FleetRootGrantAvailability,
    pub authority_match: FleetRootGrantAuthorityMatch,
    pub observed_balance: u128,
    pub requested_cycles: u128,
    pub now_ns: u64,
    pub coordinator_balance: u128,
    pub call_reservation_cycles: u128,
    pub coordinator_policy: &'a FleetCoordinatorRootFundingPolicy,
    pub root_policy: &'a FleetSubnetRootFundingPolicy,
    pub fleet_window: Option<FleetFundingWindowSnapshot>,
    pub root_window: Option<FleetFundingWindowSnapshot>,
    pub fleet_automatic_usage: FleetFundingAutomaticUsageSnapshot,
    pub root_automatic_usage: FleetFundingAutomaticUsageSnapshot,
    pub last_accepted_at_ns: Option<u64>,
}

/// Pure terminal reason for withholding a grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetRootGrantNoGrantReason {
    CooldownActive,
    CoordinatorReserveUnavailable,
    FleetAutomaticCapExhausted,
    FleetWindowExhausted,
    FundingDisabled,
    InvalidRequest,
    PolicyMismatch,
    RegistryStale,
    RootIneligible,
    RootAutomaticCapExhausted,
    RootWindowExhausted,
}

/// Pure decision for one authenticated fresh operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetRootGrantDecision {
    Grant {
        fleet_window_start_secs: u64,
        root_window_start_secs: u64,
    },
    NoGrant(FleetRootGrantNoGrantReason),
}

/// Return whether one terminal zero-transfer result permits lower-threshold ICP fallback.
#[must_use]
pub const fn permits_automatic_icp_fallback(reason: FleetRootGrantNoGrantReason) -> bool {
    matches!(
        reason,
        FleetRootGrantNoGrantReason::CoordinatorReserveUnavailable
            | FleetRootGrantNoGrantReason::FleetWindowExhausted
            | FleetRootGrantNoGrantReason::RootWindowExhausted
    )
}

/// Decide an all-or-nothing grant without reading or mutating ambient state.
#[must_use]
pub fn decide_fleet_root_grant(input: &FleetRootGrantDecisionInput<'_>) -> FleetRootGrantDecision {
    if !input.availability.funding_enabled {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::FundingDisabled);
    }
    if !input.authority_match.registry_matches {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::RegistryStale);
    }
    if !input.authority_match.policy_matches {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::PolicyMismatch);
    }
    if !input.availability.root_is_eligible {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::RootIneligible);
    }

    let root = input.root_policy;
    let Some(expected_grant) = root
        .target_balance
        .to_u128()
        .checked_sub(input.observed_balance)
    else {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::InvalidRequest);
    };
    if input.observed_balance > root.request_threshold.to_u128()
        || input.requested_cycles == 0
        || input.requested_cycles != expected_grant
    {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::InvalidRequest);
    }

    let Some(cooldown_ns) = root.cooldown_secs.checked_mul(1_000_000_000) else {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::InvalidRequest);
    };
    if input.last_accepted_at_ns.is_some_and(|accepted_at_ns| {
        accepted_at_ns
            .checked_add(cooldown_ns)
            .is_none_or(|next_at_ns| input.now_ns < next_at_ns)
    }) {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::CooldownActive);
    }

    if !automatic_usage_admits(
        input.root_automatic_usage,
        input.requested_cycles,
        root.maximum_automatic_grants,
        root.maximum_automatic_cycles.to_u128(),
    ) {
        return FleetRootGrantDecision::NoGrant(
            FleetRootGrantNoGrantReason::RootAutomaticCapExhausted,
        );
    }
    if !automatic_usage_admits(
        input.fleet_automatic_usage,
        input.requested_cycles,
        input.coordinator_policy.maximum_automatic_grants,
        input.coordinator_policy.maximum_automatic_cycles.to_u128(),
    ) {
        return FleetRootGrantDecision::NoGrant(
            FleetRootGrantNoGrantReason::FleetAutomaticCapExhausted,
        );
    }

    let now_secs = input.now_ns / 1_000_000_000;
    let Some(fleet_window_start_secs) =
        window_start(now_secs, input.coordinator_policy.budget.window_secs)
    else {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::InvalidRequest);
    };
    let Some(root_window_start_secs) = window_start(now_secs, root.budget.window_secs) else {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::InvalidRequest);
    };

    if !window_admits(
        input.fleet_window,
        fleet_window_start_secs,
        input.requested_cycles,
        input.coordinator_policy.budget.maximum_cycles.to_u128(),
    ) {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::FleetWindowExhausted);
    }
    if !window_admits(
        input.root_window,
        root_window_start_secs,
        input.requested_cycles,
        root.budget.maximum_cycles.to_u128(),
    ) {
        return FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::RootWindowExhausted);
    }

    let required_balance = input
        .requested_cycles
        .checked_add(input.call_reservation_cycles)
        .and_then(|required| {
            required.checked_add(input.coordinator_policy.minimum_reserve_cycles.to_u128())
        });
    if required_balance.is_none_or(|required| input.coordinator_balance < required) {
        return FleetRootGrantDecision::NoGrant(
            FleetRootGrantNoGrantReason::CoordinatorReserveUnavailable,
        );
    }

    FleetRootGrantDecision::Grant {
        fleet_window_start_secs,
        root_window_start_secs,
    }
}

fn window_start(now_secs: u64, window_secs: u64) -> Option<u64> {
    (window_secs != 0).then(|| (now_secs / window_secs) * window_secs)
}

fn window_admits(
    snapshot: Option<FleetFundingWindowSnapshot>,
    current_window_start_secs: u64,
    requested_cycles: u128,
    maximum_cycles: u128,
) -> bool {
    let used = snapshot
        .filter(|snapshot| snapshot.window_start_secs == current_window_start_secs)
        .and_then(|snapshot| snapshot.spent_cycles.checked_add(snapshot.reserved_cycles))
        .unwrap_or(0);
    used.checked_add(requested_cycles)
        .is_some_and(|total| total <= maximum_cycles)
}

fn automatic_usage_admits(
    usage: FleetFundingAutomaticUsageSnapshot,
    requested_cycles: u128,
    maximum_grants: u32,
    maximum_cycles: u128,
) -> bool {
    usage.successful_grants < maximum_grants
        && usage
            .granted_cycles
            .checked_add(requested_cycles)
            .is_some_and(|total| total <= maximum_cycles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        ids::{CyclesFundingBudget, FleetFundingProfile, FleetSubnetRootFundingPolicy},
    };

    #[test]
    fn grants_exact_target_and_charges_reservation_time_windows() {
        let coordinator = coordinator_policy();
        let root = root_policy();
        let input = input(&coordinator, &root);
        assert_eq!(
            decide_fleet_root_grant(&input),
            FleetRootGrantDecision::Grant {
                fleet_window_start_secs: 3_600,
                root_window_start_secs: 3_600,
            }
        );
    }

    #[test]
    fn rejects_partial_budget_reserve_cooldown_and_stale_authority() {
        let coordinator = coordinator_policy();
        let root = root_policy();

        let mut value = input(&coordinator, &root);
        value.requested_cycles -= 1;
        assert_eq!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::InvalidRequest)
        );

        let mut value = input(&coordinator, &root);
        value.fleet_window = Some(FleetFundingWindowSnapshot {
            window_start_secs: 3_600,
            spent_cycles: 50_000_000_001,
            reserved_cycles: 0,
        });
        assert_eq!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::FleetWindowExhausted)
        );

        let mut value = input(&coordinator, &root);
        value.coordinator_balance = 102_000_000_000;
        assert_eq!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::NoGrant(
                FleetRootGrantNoGrantReason::CoordinatorReserveUnavailable
            )
        );

        let mut value = input(&coordinator, &root);
        value.last_accepted_at_ns = Some(value.now_ns - 1);
        assert_eq!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::CooldownActive)
        );

        let mut value = input(&coordinator, &root);
        value.authority_match.registry_matches = false;
        assert_eq!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::NoGrant(FleetRootGrantNoGrantReason::RegistryStale)
        );
    }

    #[test]
    fn rejects_each_fail_closed_authority_and_availability_case() {
        let coordinator = coordinator_policy();
        let root = root_policy();

        let mut value = input(&coordinator, &root);
        value.availability.funding_enabled = false;
        assert_no_grant(&value, FleetRootGrantNoGrantReason::FundingDisabled);

        let mut value = input(&coordinator, &root);
        value.authority_match.policy_matches = false;
        assert_no_grant(&value, FleetRootGrantNoGrantReason::PolicyMismatch);

        let mut value = input(&coordinator, &root);
        value.availability.root_is_eligible = false;
        assert_no_grant(&value, FleetRootGrantNoGrantReason::RootIneligible);

        let mut value = input(&coordinator, &root);
        value.observed_balance = root.request_threshold.to_u128() + 1;
        assert_no_grant(&value, FleetRootGrantNoGrantReason::InvalidRequest);

        let mut value = input(&coordinator, &root);
        value.root_window = Some(FleetFundingWindowSnapshot {
            window_start_secs: 3_600,
            spent_cycles: 50_000_000_001,
            reserved_cycles: 0,
        });
        assert_no_grant(&value, FleetRootGrantNoGrantReason::RootWindowExhausted);
    }

    #[test]
    fn nonrenewing_caps_remain_terminal_after_rolling_window_changes() {
        let coordinator = coordinator_policy();
        let root = root_policy();

        let mut value = input(&coordinator, &root);
        value.now_ns = 99_999_999_999_999;
        value.root_automatic_usage.successful_grants = root.maximum_automatic_grants;
        assert_no_grant(
            &value,
            FleetRootGrantNoGrantReason::RootAutomaticCapExhausted,
        );

        let mut value = input(&coordinator, &root);
        value.now_ns = 99_999_999_999_999;
        value.fleet_automatic_usage.granted_cycles = coordinator.maximum_automatic_cycles.to_u128();
        assert_no_grant(
            &value,
            FleetRootGrantNoGrantReason::FleetAutomaticCapExhausted,
        );
    }

    #[test]
    fn admits_exact_reserve_cooldown_and_window_boundaries() {
        let coordinator = coordinator_policy();
        let root = root_policy();
        let mut value = input(&coordinator, &root);
        value.fleet_window = Some(FleetFundingWindowSnapshot {
            window_start_secs: 3_600,
            spent_cycles: 25_000_000_000,
            reserved_cycles: 25_000_000_000,
        });
        value.root_window = value.fleet_window;
        value.last_accepted_at_ns = Some(value.now_ns - 300_000_000_000);
        value.coordinator_balance = value.requested_cycles
            + value.call_reservation_cycles
            + coordinator.minimum_reserve_cycles.to_u128();
        assert!(matches!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::Grant { .. }
        ));

        value.coordinator_balance -= 1;
        assert_no_grant(
            &value,
            FleetRootGrantNoGrantReason::CoordinatorReserveUnavailable,
        );

        value.coordinator_balance += 1;
        value.now_ns = 7_200_000_000_000;
        assert_eq!(
            decide_fleet_root_grant(&value),
            FleetRootGrantDecision::Grant {
                fleet_window_start_secs: 7_200,
                root_window_start_secs: 7_200,
            }
        );
    }

    fn coordinator_policy() -> FleetCoordinatorRootFundingPolicy {
        FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            minimum_reserve_cycles: Cycles::new(50_000_000_000),
            budget: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(100_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(200_000_000_000),
        }
    }

    fn root_policy() -> FleetSubnetRootFundingPolicy {
        FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            request_threshold: Cycles::new(50_000_000_000),
            target_balance: Cycles::new(60_000_000_000),
            cooldown_secs: 300,
            budget: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(100_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(200_000_000_000),
        }
    }

    fn input<'a>(
        coordinator_policy: &'a FleetCoordinatorRootFundingPolicy,
        root_policy: &'a FleetSubnetRootFundingPolicy,
    ) -> FleetRootGrantDecisionInput<'a> {
        FleetRootGrantDecisionInput {
            availability: FleetRootGrantAvailability {
                funding_enabled: true,
                root_is_eligible: true,
            },
            authority_match: FleetRootGrantAuthorityMatch {
                registry_matches: true,
                policy_matches: true,
            },
            observed_balance: 10_000_000_000,
            requested_cycles: 50_000_000_000,
            now_ns: 3_700_000_000_000,
            coordinator_balance: 200_000_000_000,
            call_reservation_cycles: 42_118_809_000,
            coordinator_policy,
            root_policy,
            fleet_window: None,
            root_window: None,
            fleet_automatic_usage: FleetFundingAutomaticUsageSnapshot {
                successful_grants: 0,
                granted_cycles: 0,
            },
            root_automatic_usage: FleetFundingAutomaticUsageSnapshot {
                successful_grants: 0,
                granted_cycles: 0,
            },
            last_accepted_at_ns: None,
        }
    }

    fn assert_no_grant(
        input: &FleetRootGrantDecisionInput<'_>,
        reason: FleetRootGrantNoGrantReason,
    ) {
        assert_eq!(
            decide_fleet_root_grant(input),
            FleetRootGrantDecision::NoGrant(reason)
        );
    }
}
