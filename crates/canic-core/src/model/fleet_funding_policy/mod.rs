//! Module: model::fleet_funding_policy
//!
//! Responsibility: own immutable Fleet root-funding policy invariants.
//! Does not own: config decoding, canonical hashing, storage, accounting, or effects.
//! Boundary: host and canister admission validate the same protected policy shapes.

use crate::ids::{
    COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES,
    FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES, FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES,
    FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingAuthority,
};
use candid::Principal;
use thiserror::Error as ThisError;

/// Exact invariant rejected while admitting immutable Fleet funding policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetFundingPolicyValidationError {
    #[error("Coordinator minimum reserve must be positive")]
    CoordinatorReserveZero,
    #[error("Coordinator minimum reserve is below the measured execution floor")]
    CoordinatorReserveBelowFloor,
    #[error("Coordinator accounting window must be positive")]
    CoordinatorWindowZero,
    #[error("Coordinator Fleet-wide budget must be positive")]
    CoordinatorMaximumZero,
    #[error("Coordinator Fleet-wide budget cannot admit the largest root target")]
    CoordinatorMaximumBelowLargestRootTarget,
    #[error("root request threshold must be positive")]
    RootRequestThresholdZero,
    #[error("root request threshold is below the measured request/recovery floor")]
    RootRequestThresholdBelowFloor,
    #[error("root target balance must be positive")]
    RootTargetBalanceZero,
    #[error("root target balance must exceed its request threshold")]
    RootTargetNotAboveRequestThreshold,
    #[error("root cooldown must be positive")]
    RootCooldownZero,
    #[error("root accounting window must be positive")]
    RootWindowZero,
    #[error("root budget must be positive")]
    RootMaximumZero,
    #[error("root budget cannot admit its largest legitimate zero-balance grant")]
    RootMaximumBelowTargetBalance,
    #[error("ICP per-call cap must be positive")]
    IcpPerCallMaximumZero,
    #[error("ICP accounting window must be positive")]
    IcpWindowZero,
    #[error("ICP cumulative budget must be positive")]
    IcpMaximumZero,
    #[error("ICP cumulative budget must admit one maximum per-call refill")]
    IcpMaximumBelowPerCallMaximum,
    #[error("ICP retained-balance floor must be positive")]
    IcpMinimumBalanceZero,
    #[error("minimum ICP/XDR rate must be positive when present")]
    IcpMinimumRateZero,
    #[error("ICP Ledger override must name a non-reserved Canister principal")]
    IcpLedgerPrincipalReserved,
    #[error("CMC override must name a non-reserved Canister principal")]
    IcpCmcPrincipalReserved,
    #[error("IC system-Canister overrides require the explicit safety acknowledgement")]
    IcpOverrideUnsafe,
    #[error("automatic emergency threshold must be positive")]
    AutomaticEmergencyThresholdZero,
    #[error("automatic emergency threshold is below the measured execution/recovery floor")]
    AutomaticEmergencyThresholdBelowFloor,
    #[error("automatic emergency threshold must be below the Coordinator request threshold")]
    AutomaticEmergencyNotBelowRequestThreshold,
    #[error("automatic target balance must be positive")]
    AutomaticTargetBalanceZero,
    #[error("automatic target balance must exceed the Coordinator request threshold")]
    AutomaticTargetNotAboveRequestThreshold,
    #[error("automatic target balance must not exceed the Coordinator target balance")]
    AutomaticTargetAboveRootTargetBalance,
}

/// Validate one immutable Coordinator treasury policy.
pub const fn validate_coordinator_root_funding_policy(
    policy: &FleetCoordinatorRootFundingPolicy,
) -> Result<(), FleetFundingPolicyValidationError> {
    let reserve = policy.minimum_reserve_cycles.to_u128();
    if reserve == 0 {
        return Err(FleetFundingPolicyValidationError::CoordinatorReserveZero);
    }
    if reserve < COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES {
        return Err(FleetFundingPolicyValidationError::CoordinatorReserveBelowFloor);
    }
    if policy.budget.window_secs == 0 {
        return Err(FleetFundingPolicyValidationError::CoordinatorWindowZero);
    }
    if policy.budget.maximum_cycles.to_u128() == 0 {
        return Err(FleetFundingPolicyValidationError::CoordinatorMaximumZero);
    }
    Ok(())
}

/// Validate one root's complete immutable funding authority.
pub fn validate_fleet_subnet_root_funding_authority(
    authority: &FleetSubnetRootFundingAuthority,
    ic_mainnet: bool,
) -> Result<(), FleetFundingPolicyValidationError> {
    let root = &authority.root_funding;
    let request_threshold = root.request_threshold.to_u128();
    let target_balance = root.target_balance.to_u128();
    let maximum_cycles = root.budget.maximum_cycles.to_u128();
    if request_threshold == 0 {
        return Err(FleetFundingPolicyValidationError::RootRequestThresholdZero);
    }
    if request_threshold < FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES {
        return Err(FleetFundingPolicyValidationError::RootRequestThresholdBelowFloor);
    }
    if target_balance == 0 {
        return Err(FleetFundingPolicyValidationError::RootTargetBalanceZero);
    }
    if target_balance <= request_threshold {
        return Err(FleetFundingPolicyValidationError::RootTargetNotAboveRequestThreshold);
    }
    if root.cooldown_secs == 0 {
        return Err(FleetFundingPolicyValidationError::RootCooldownZero);
    }
    if root.budget.window_secs == 0 {
        return Err(FleetFundingPolicyValidationError::RootWindowZero);
    }
    if maximum_cycles == 0 {
        return Err(FleetFundingPolicyValidationError::RootMaximumZero);
    }
    if maximum_cycles < target_balance {
        return Err(FleetFundingPolicyValidationError::RootMaximumBelowTargetBalance);
    }

    let Some(icp) = authority.icp_refill.as_ref() else {
        return Ok(());
    };
    if icp.max_refill_e8s_per_call == 0 {
        return Err(FleetFundingPolicyValidationError::IcpPerCallMaximumZero);
    }
    if icp.window_secs == 0 {
        return Err(FleetFundingPolicyValidationError::IcpWindowZero);
    }
    if icp.maximum_refill_e8s == 0 {
        return Err(FleetFundingPolicyValidationError::IcpMaximumZero);
    }
    if icp.maximum_refill_e8s < icp.max_refill_e8s_per_call {
        return Err(FleetFundingPolicyValidationError::IcpMaximumBelowPerCallMaximum);
    }
    if icp.minimum_icp_balance_e8s == 0 {
        return Err(FleetFundingPolicyValidationError::IcpMinimumBalanceZero);
    }
    if icp.min_xdr_permyriad_per_icp == Some(0) {
        return Err(FleetFundingPolicyValidationError::IcpMinimumRateZero);
    }
    if icp.ledger_canister_id.is_some_and(principal_is_reserved) {
        return Err(FleetFundingPolicyValidationError::IcpLedgerPrincipalReserved);
    }
    if icp.cmc_canister_id.is_some_and(principal_is_reserved) {
        return Err(FleetFundingPolicyValidationError::IcpCmcPrincipalReserved);
    }
    if ic_mainnet
        && (icp.ledger_canister_id.is_some() || icp.cmc_canister_id.is_some())
        && !icp.allow_ic_system_canister_overrides
    {
        return Err(FleetFundingPolicyValidationError::IcpOverrideUnsafe);
    }

    let Some(automatic) = icp.automatic.as_ref() else {
        return Ok(());
    };
    let emergency_threshold = automatic.emergency_threshold.to_u128();
    let automatic_target = automatic.target_balance.to_u128();
    if emergency_threshold == 0 {
        return Err(FleetFundingPolicyValidationError::AutomaticEmergencyThresholdZero);
    }
    if emergency_threshold < FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES {
        return Err(FleetFundingPolicyValidationError::AutomaticEmergencyThresholdBelowFloor);
    }
    if emergency_threshold >= request_threshold {
        return Err(FleetFundingPolicyValidationError::AutomaticEmergencyNotBelowRequestThreshold);
    }
    if automatic_target == 0 {
        return Err(FleetFundingPolicyValidationError::AutomaticTargetBalanceZero);
    }
    if automatic_target <= request_threshold {
        return Err(FleetFundingPolicyValidationError::AutomaticTargetNotAboveRequestThreshold);
    }
    if automatic_target > target_balance {
        return Err(FleetFundingPolicyValidationError::AutomaticTargetAboveRootTargetBalance);
    }
    Ok(())
}

/// Validate that one Fleet-wide budget admits every root's maximum legitimate grant.
pub fn validate_fleet_root_funding_capacity<'a>(
    coordinator: &FleetCoordinatorRootFundingPolicy,
    roots: impl IntoIterator<Item = &'a FleetSubnetRootFundingAuthority>,
) -> Result<(), FleetFundingPolicyValidationError> {
    let largest_target = roots
        .into_iter()
        .map(|root| root.root_funding.target_balance.to_u128())
        .max()
        .unwrap_or(0);
    if coordinator.budget.maximum_cycles.to_u128() < largest_target {
        Err(FleetFundingPolicyValidationError::CoordinatorMaximumBelowLargestRootTarget)
    } else {
        Ok(())
    }
}

fn principal_is_reserved(principal: Principal) -> bool {
    principal == Principal::anonymous() || principal == Principal::management_canister()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        ids::{CyclesFundingBudget, FleetSubnetRootFundingPolicy},
    };

    #[test]
    fn root_policy_validation_rejects_each_threshold_boundary() {
        let authority = authority();
        validate_fleet_subnet_root_funding_authority(&authority, false)
            .expect("valid root authority");

        let mut changed = authority.clone();
        changed.root_funding.request_threshold =
            Cycles::new(FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES.saturating_sub(1));
        assert_eq!(
            validate_fleet_subnet_root_funding_authority(&changed, false),
            Err(FleetFundingPolicyValidationError::RootRequestThresholdBelowFloor)
        );

        let mut changed = authority;
        changed.root_funding.target_balance = changed.root_funding.request_threshold.clone();
        assert_eq!(
            validate_fleet_subnet_root_funding_authority(&changed, false),
            Err(FleetFundingPolicyValidationError::RootTargetNotAboveRequestThreshold)
        );
    }

    fn authority() -> FleetSubnetRootFundingAuthority {
        FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                request_threshold: Cycles::new(50_000_000_000),
                target_balance: Cycles::new(60_000_000_000),
                cooldown_secs: 300,
                budget: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(100_000_000_000),
                },
            },
            icp_refill: None,
        }
    }
}
