//! Module: model::fleet_funding_policy
//!
//! Responsibility: own immutable Fleet root-funding policy invariants.
//! Does not own: config decoding, canonical hashing, storage, accounting, or effects.
//! Boundary: host and canister admission validate the same protected policy shapes.

use crate::ids::{
    COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES,
    FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES, FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES,
    FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingAuthority,
    FleetSubnetRootFundingPolicy, MAX_FLEET_ROOT_FUNDING_SLOTS, SubnetId,
};
use candid::Principal;
use thiserror::Error as ThisError;

const TRILLION_CYCLES: u128 = 1_000_000_000_000;
const THIRTY_DAYS_SECS: u64 = 30 * 24 * 60 * 60;
const NINETY_DAYS_SECS: u64 = 90 * 24 * 60 * 60;
const MAXIMUM_AUTOMATIC_EVENTS: u32 = 4;

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
    #[error("Coordinator funding profile differs from a root funding profile")]
    CoordinatorProfileMismatch,
    #[error("Coordinator minimum reserve is below the funding-profile baseline")]
    CoordinatorReserveBelowProfileBaseline,
    #[error("Coordinator accounting window is below the funding-profile baseline")]
    CoordinatorWindowBelowProfileBaseline,
    #[error("Coordinator non-renewing automatic grant count must be positive")]
    CoordinatorAutomaticGrantCountZero,
    #[error("Coordinator non-renewing automatic grant count exceeds admitted root authority")]
    CoordinatorAutomaticGrantCountAboveRoots,
    #[error("Coordinator non-renewing automatic cycles must be positive")]
    CoordinatorAutomaticCyclesZero,
    #[error("Coordinator non-renewing automatic cycles cannot admit the largest root target")]
    CoordinatorAutomaticCyclesBelowLargestRootTarget,
    #[error("Coordinator non-renewing automatic cycles exceed admitted root authority")]
    CoordinatorAutomaticCyclesAboveRoots,
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
    #[error("root request threshold is below the funding-profile baseline")]
    RootRequestThresholdBelowProfileBaseline,
    #[error("root target balance is below the funding-profile baseline")]
    RootTargetBelowProfileBaseline,
    #[error("root target/threshold gap is below the funding-profile baseline")]
    RootTargetGapBelowProfileBaseline,
    #[error("root cooldown is below the funding-profile baseline")]
    RootCooldownBelowProfileBaseline,
    #[error("root accounting window is below the funding-profile baseline")]
    RootWindowBelowProfileBaseline,
    #[error("root rolling window can admit two minimum threshold-triggered grants")]
    RootWindowAdmitsTwoMinimumGrants,
    #[error("root non-renewing automatic grant count must be between one and four")]
    RootAutomaticGrantCountInvalid,
    #[error("root non-renewing automatic cycles must be positive")]
    RootAutomaticCyclesZero,
    #[error("root non-renewing automatic cycles cannot admit one largest valid grant")]
    RootAutomaticCyclesBelowTargetBalance,
    #[error("root non-renewing automatic cycles exceed the count-bound reachable maximum")]
    RootAutomaticCyclesAboveReachableMaximum,
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
    #[error("automatic ICP refill count must be between one and four")]
    AutomaticIcpRefillCountInvalid,
    #[error("automatic ICP refill spend cap must be positive")]
    AutomaticIcpRefillMaximumZero,
    #[error("automatic ICP refill spend cap must admit one maximum per-call refill")]
    AutomaticIcpRefillMaximumBelowPerCallMaximum,
}

/// Exact invariant rejected for one no-effect funding-policy rotation plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetFundingPolicyRotationValidationError {
    #[error("funding-policy rotation generation is not the exact monotonic successor")]
    GenerationMismatch,
    #[error("funding-policy rotation must contain one to 4,096 exact Roots")]
    RootCountInvalid,
    #[error("funding-policy rotation Roots are not strictly ordered and unique")]
    RootOrderInvalid,
    #[error("funding-policy rotation retained usage is inconsistent")]
    UsageMismatch,
    #[error("funding-policy rotation maximum exposure is inconsistent")]
    ExposureMismatch,
    #[error("funding-policy rotation apply must have zero immediate operator debit")]
    OperatorDebitNonzero,
    #[error("funding-policy rotation placement evidence is invalid")]
    PlacementEvidenceInvalid,
    #[error("funding-policy rotation profile differs from physical topology")]
    ProfileTopologyMismatch,
    #[error("funding-policy rotation contains invalid funding policy")]
    PolicyInvalid,
}

/// DTO-free cumulative funding usage supplied to the model-owned rotation validator.
pub struct FleetFundingPolicyRotationUsageInput {
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: u128,
    pub generation_automatic_grants: u32,
    pub generation_automatic_cycles: u128,
}

/// DTO-free physical placement supplied to the model-owned rotation validator.
pub struct FleetFundingPolicyRotationPlacementInput {
    pub subnet: SubnetId,
    pub node_count: u64,
    pub cost_multiplier_numerator: u64,
    pub cost_multiplier_denominator: u64,
    pub fiduciary: bool,
    pub acknowledge_fiduciary_cost: bool,
}

/// DTO-free proposed Root policy supplied to the model-owned rotation validator.
pub struct FleetFundingPolicyRotationRootInput<'a> {
    pub fleet_subnet_root: Principal,
    pub predecessor_usage: FleetFundingPolicyRotationUsageInput,
    pub proposed_policy: &'a FleetSubnetRootFundingPolicy,
    pub placement: FleetFundingPolicyRotationPlacementInput,
}

/// Complete DTO-free evidence for one model-owned funding-policy rotation decision.
pub struct FleetFundingPolicyRotationPlanInput<'a> {
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub predecessor_usage: FleetFundingPolicyRotationUsageInput,
    pub proposed_coordinator_policy: &'a FleetCoordinatorRootFundingPolicy,
    pub coordinator_placement: FleetFundingPolicyRotationPlacementInput,
    pub affected_root_count: u32,
    pub maximum_new_automatic_cycles: u128,
    pub apply_operator_debit: u128,
    pub funding_source_is_coordinator_treasury: bool,
    pub roots: Vec<FleetFundingPolicyRotationRootInput<'a>>,
}

/// Validate the pure bounded and economic invariants of one complete rotation plan.
pub fn validate_fleet_funding_policy_rotation(
    plan: &FleetFundingPolicyRotationPlanInput<'_>,
) -> Result<(), FleetFundingPolicyRotationValidationError> {
    if plan.predecessor_generation == 0
        || plan
            .predecessor_generation
            .checked_add(1)
            .is_none_or(|successor| successor != plan.successor_generation)
    {
        return Err(FleetFundingPolicyRotationValidationError::GenerationMismatch);
    }
    if plan.roots.is_empty()
        || plan.roots.len() > MAX_FLEET_ROOT_FUNDING_SLOTS
        || usize::try_from(plan.affected_root_count).ok() != Some(plan.roots.len())
    {
        return Err(FleetFundingPolicyRotationValidationError::RootCountInvalid);
    }
    if plan
        .roots
        .windows(2)
        .any(|roots| roots[0].fleet_subnet_root >= roots[1].fleet_subnet_root)
    {
        return Err(FleetFundingPolicyRotationValidationError::RootOrderInvalid);
    }
    if plan.maximum_new_automatic_cycles
        != plan
            .proposed_coordinator_policy
            .maximum_automatic_cycles
            .to_u128()
    {
        return Err(FleetFundingPolicyRotationValidationError::ExposureMismatch);
    }
    if plan.apply_operator_debit != 0 || !plan.funding_source_is_coordinator_treasury {
        return Err(FleetFundingPolicyRotationValidationError::OperatorDebitNonzero);
    }
    validate_rotation_placement(&plan.coordinator_placement)?;
    validate_coordinator_root_funding_policy(plan.proposed_coordinator_policy)
        .map_err(|_| FleetFundingPolicyRotationValidationError::PolicyInvalid)?;

    let mut authorities = Vec::with_capacity(plan.roots.len());
    let mut historical_grants = 0_u64;
    let mut historical_cycles = 0_u128;
    let mut generation_grants = 0_u32;
    let mut generation_cycles = 0_u128;
    for root in &plan.roots {
        validate_rotation_placement(&root.placement)?;
        let authority = FleetSubnetRootFundingAuthority {
            root_funding: root.proposed_policy.clone(),
            icp_refill: None,
        };
        validate_fleet_subnet_root_funding_authority(&authority, false)
            .map_err(|_| FleetFundingPolicyRotationValidationError::PolicyInvalid)?;
        historical_grants = historical_grants
            .checked_add(root.predecessor_usage.historical_automatic_grants)
            .ok_or(FleetFundingPolicyRotationValidationError::UsageMismatch)?;
        historical_cycles = historical_cycles
            .checked_add(root.predecessor_usage.historical_automatic_cycles)
            .ok_or(FleetFundingPolicyRotationValidationError::UsageMismatch)?;
        generation_grants = generation_grants
            .checked_add(root.predecessor_usage.generation_automatic_grants)
            .ok_or(FleetFundingPolicyRotationValidationError::UsageMismatch)?;
        generation_cycles = generation_cycles
            .checked_add(root.predecessor_usage.generation_automatic_cycles)
            .ok_or(FleetFundingPolicyRotationValidationError::UsageMismatch)?;
        authorities.push(authority);
    }
    let usage = &plan.predecessor_usage;
    if usage.historical_automatic_grants != historical_grants
        || usage.historical_automatic_cycles != historical_cycles
        || usage.generation_automatic_grants != generation_grants
        || usage.generation_automatic_cycles != generation_cycles
    {
        return Err(FleetFundingPolicyRotationValidationError::UsageMismatch);
    }
    validate_fleet_root_funding_capacity(plan.proposed_coordinator_policy, authorities.iter())
        .map_err(|_| FleetFundingPolicyRotationValidationError::PolicyInvalid)?;
    let crosses_subnets = plan
        .roots
        .iter()
        .any(|root| root.placement.subnet != plan.coordinator_placement.subnet);
    let profile_matches = match plan.proposed_coordinator_policy.funding_profile {
        FleetFundingProfile::SingleSubnet => !crosses_subnets,
        FleetFundingProfile::PreviewMultiSubnet | FleetFundingProfile::MultiSubnet => {
            crosses_subnets
        }
    };
    if !profile_matches {
        return Err(FleetFundingPolicyRotationValidationError::ProfileTopologyMismatch);
    }
    Ok(())
}

const fn validate_rotation_placement(
    placement: &FleetFundingPolicyRotationPlacementInput,
) -> Result<(), FleetFundingPolicyRotationValidationError> {
    let acknowledgement_matches = placement.fiduciary == placement.acknowledge_fiduciary_cost;
    if placement.node_count == 0
        || placement.cost_multiplier_numerator == 0
        || placement.cost_multiplier_denominator == 0
        || !acknowledgement_matches
    {
        return Err(FleetFundingPolicyRotationValidationError::PlacementEvidenceInvalid);
    }
    Ok(())
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
    if policy.budget.window_secs < NINETY_DAYS_SECS {
        return Err(FleetFundingPolicyValidationError::CoordinatorWindowBelowProfileBaseline);
    }
    if policy.minimum_reserve_cycles.to_u128() < profile_reserve(policy.funding_profile) {
        return Err(FleetFundingPolicyValidationError::CoordinatorReserveBelowProfileBaseline);
    }
    if policy.maximum_automatic_grants == 0 {
        return Err(FleetFundingPolicyValidationError::CoordinatorAutomaticGrantCountZero);
    }
    if policy.maximum_automatic_cycles.to_u128() == 0 {
        return Err(FleetFundingPolicyValidationError::CoordinatorAutomaticCyclesZero);
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
    let (minimum_threshold, minimum_target) = profile_root_balances(root.funding_profile);
    validate_root_balance_policy(root, minimum_threshold, minimum_target)?;

    let Some(icp) = authority.icp_refill.as_ref() else {
        return Ok(());
    };
    validate_icp_refill_policy(icp, ic_mainnet)?;

    let Some(automatic) = icp.automatic.as_ref() else {
        return Ok(());
    };
    validate_automatic_icp_refill_policy(
        automatic,
        request_threshold,
        target_balance,
        icp.max_refill_e8s_per_call,
    )
}

fn validate_root_balance_policy(
    root: &FleetSubnetRootFundingPolicy,
    minimum_threshold: u128,
    minimum_target: u128,
) -> Result<(), FleetFundingPolicyValidationError> {
    let request_threshold = root.request_threshold.to_u128();
    let target_balance = root.target_balance.to_u128();
    let maximum_cycles = root.budget.maximum_cycles.to_u128();
    if request_threshold == 0 {
        return Err(FleetFundingPolicyValidationError::RootRequestThresholdZero);
    }
    if request_threshold < FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES {
        return Err(FleetFundingPolicyValidationError::RootRequestThresholdBelowFloor);
    }
    if request_threshold < minimum_threshold {
        return Err(FleetFundingPolicyValidationError::RootRequestThresholdBelowProfileBaseline);
    }
    if target_balance == 0 {
        return Err(FleetFundingPolicyValidationError::RootTargetBalanceZero);
    }
    if target_balance <= request_threshold {
        return Err(FleetFundingPolicyValidationError::RootTargetNotAboveRequestThreshold);
    }
    if target_balance < minimum_target {
        return Err(FleetFundingPolicyValidationError::RootTargetBelowProfileBaseline);
    }
    let target_gap = target_balance - request_threshold;
    if target_gap < minimum_target - minimum_threshold {
        return Err(FleetFundingPolicyValidationError::RootTargetGapBelowProfileBaseline);
    }
    if root.cooldown_secs == 0 {
        return Err(FleetFundingPolicyValidationError::RootCooldownZero);
    }
    if root.cooldown_secs < THIRTY_DAYS_SECS {
        return Err(FleetFundingPolicyValidationError::RootCooldownBelowProfileBaseline);
    }
    if root.budget.window_secs == 0 {
        return Err(FleetFundingPolicyValidationError::RootWindowZero);
    }
    if root.budget.window_secs < NINETY_DAYS_SECS {
        return Err(FleetFundingPolicyValidationError::RootWindowBelowProfileBaseline);
    }
    if maximum_cycles == 0 {
        return Err(FleetFundingPolicyValidationError::RootMaximumZero);
    }
    if maximum_cycles < target_balance {
        return Err(FleetFundingPolicyValidationError::RootMaximumBelowTargetBalance);
    }
    if target_gap
        .checked_mul(2)
        .is_none_or(|two_grants| maximum_cycles >= two_grants)
    {
        return Err(FleetFundingPolicyValidationError::RootWindowAdmitsTwoMinimumGrants);
    }
    if root.maximum_automatic_grants == 0
        || root.maximum_automatic_grants > MAXIMUM_AUTOMATIC_EVENTS
    {
        return Err(FleetFundingPolicyValidationError::RootAutomaticGrantCountInvalid);
    }
    let maximum_automatic_cycles = root.maximum_automatic_cycles.to_u128();
    if maximum_automatic_cycles == 0 {
        return Err(FleetFundingPolicyValidationError::RootAutomaticCyclesZero);
    }
    if maximum_automatic_cycles < target_balance {
        return Err(FleetFundingPolicyValidationError::RootAutomaticCyclesBelowTargetBalance);
    }
    if target_balance
        .checked_mul(u128::from(root.maximum_automatic_grants))
        .is_none_or(|reachable| maximum_automatic_cycles > reachable)
    {
        return Err(FleetFundingPolicyValidationError::RootAutomaticCyclesAboveReachableMaximum);
    }

    Ok(())
}

fn validate_icp_refill_policy(
    icp: &crate::ids::FleetSubnetRootIcpRefillPolicy,
    ic_mainnet: bool,
) -> Result<(), FleetFundingPolicyValidationError> {
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

    Ok(())
}

const fn validate_automatic_icp_refill_policy(
    automatic: &crate::ids::FleetSubnetRootAutomaticIcpRefillPolicy,
    request_threshold: u128,
    target_balance: u128,
    max_refill_e8s_per_call: u64,
) -> Result<(), FleetFundingPolicyValidationError> {
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
    if automatic.maximum_automatic_refills == 0
        || automatic.maximum_automatic_refills > MAXIMUM_AUTOMATIC_EVENTS
    {
        return Err(FleetFundingPolicyValidationError::AutomaticIcpRefillCountInvalid);
    }
    if automatic.maximum_automatic_refill_e8s == 0 {
        return Err(FleetFundingPolicyValidationError::AutomaticIcpRefillMaximumZero);
    }
    if automatic.maximum_automatic_refill_e8s < max_refill_e8s_per_call {
        return Err(
            FleetFundingPolicyValidationError::AutomaticIcpRefillMaximumBelowPerCallMaximum,
        );
    }
    Ok(())
}

/// Validate that one Fleet-wide budget admits every root's maximum legitimate grant.
pub fn validate_fleet_root_funding_capacity<'a>(
    coordinator: &FleetCoordinatorRootFundingPolicy,
    roots: impl IntoIterator<Item = &'a FleetSubnetRootFundingAuthority>,
) -> Result<(), FleetFundingPolicyValidationError> {
    let roots = roots.into_iter().collect::<Vec<_>>();
    for root in &roots {
        validate_fleet_root_funding_admission(coordinator, root)?;
    }
    let root_grants = roots.iter().try_fold(0_u32, |total, root| {
        total.checked_add(root.root_funding.maximum_automatic_grants)
    });
    if root_grants.is_none_or(|total| coordinator.maximum_automatic_grants > total) {
        return Err(FleetFundingPolicyValidationError::CoordinatorAutomaticGrantCountAboveRoots);
    }
    let root_cycles = roots.iter().try_fold(0_u128, |total, root| {
        total.checked_add(root.root_funding.maximum_automatic_cycles.to_u128())
    });
    if root_cycles.is_none_or(|total| coordinator.maximum_automatic_cycles.to_u128() > total) {
        return Err(FleetFundingPolicyValidationError::CoordinatorAutomaticCyclesAboveRoots);
    }
    Ok(())
}

/// Validate the Fleet-wide limits that must admit one Root before Registry activation.
pub fn validate_fleet_root_funding_admission(
    coordinator: &FleetCoordinatorRootFundingPolicy,
    root: &FleetSubnetRootFundingAuthority,
) -> Result<(), FleetFundingPolicyValidationError> {
    if root.root_funding.funding_profile != coordinator.funding_profile {
        return Err(FleetFundingPolicyValidationError::CoordinatorProfileMismatch);
    }
    let target = root.root_funding.target_balance.to_u128();
    if coordinator.budget.maximum_cycles.to_u128() < target {
        return Err(FleetFundingPolicyValidationError::CoordinatorMaximumBelowLargestRootTarget);
    }
    if coordinator.maximum_automatic_cycles.to_u128() < target {
        return Err(
            FleetFundingPolicyValidationError::CoordinatorAutomaticCyclesBelowLargestRootTarget,
        );
    }
    Ok(())
}

const fn profile_reserve(profile: FleetFundingProfile) -> u128 {
    match profile {
        FleetFundingProfile::SingleSubnet => 30 * TRILLION_CYCLES,
        FleetFundingProfile::PreviewMultiSubnet => 80 * TRILLION_CYCLES,
        FleetFundingProfile::MultiSubnet => 2_000 * TRILLION_CYCLES,
    }
}

const fn profile_root_balances(profile: FleetFundingProfile) -> (u128, u128) {
    match profile {
        FleetFundingProfile::SingleSubnet | FleetFundingProfile::PreviewMultiSubnet => {
            (10 * TRILLION_CYCLES, 30 * TRILLION_CYCLES)
        }
        FleetFundingProfile::MultiSubnet => (250 * TRILLION_CYCLES, 1_000 * TRILLION_CYCLES),
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
        ids::{CyclesFundingBudget, FleetFundingProfile, FleetSubnetRootFundingPolicy},
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

    #[test]
    fn root_policy_rejects_invalid_nonrenewing_authority() {
        let mut changed = authority();
        changed.root_funding.maximum_automatic_grants = 0;
        assert_eq!(
            validate_fleet_subnet_root_funding_authority(&changed, false),
            Err(FleetFundingPolicyValidationError::RootAutomaticGrantCountInvalid)
        );

        let mut changed = authority();
        changed.root_funding.maximum_automatic_cycles =
            Cycles::new(changed.root_funding.target_balance.to_u128() - 1);
        assert_eq!(
            validate_fleet_subnet_root_funding_authority(&changed, false),
            Err(FleetFundingPolicyValidationError::RootAutomaticCyclesBelowTargetBalance)
        );

        let mut changed = authority();
        changed.root_funding.maximum_automatic_cycles = Cycles::new(
            changed.root_funding.target_balance.to_u128()
                * u128::from(changed.root_funding.maximum_automatic_grants)
                + 1,
        );
        assert_eq!(
            validate_fleet_subnet_root_funding_authority(&changed, false),
            Err(FleetFundingPolicyValidationError::RootAutomaticCyclesAboveReachableMaximum)
        );
    }

    #[test]
    fn fleet_capacity_binds_profiles_and_nonrenewing_caps() {
        let root = authority();
        let coordinator = coordinator_policy();
        validate_fleet_root_funding_capacity(&coordinator, [&root])
            .expect("one standard Root fits the Coordinator policy");

        let mut changed = root.clone();
        changed.root_funding.funding_profile = FleetFundingProfile::MultiSubnet;
        assert_eq!(
            validate_fleet_root_funding_capacity(&coordinator, [&changed]),
            Err(FleetFundingPolicyValidationError::CoordinatorProfileMismatch)
        );

        let mut changed = coordinator.clone();
        changed.maximum_automatic_grants = 5;
        assert_eq!(
            validate_fleet_root_funding_capacity(&changed, [&root]),
            Err(FleetFundingPolicyValidationError::CoordinatorAutomaticGrantCountAboveRoots)
        );

        let mut changed = coordinator;
        changed.maximum_automatic_cycles = Cycles::new(120_000_000_000_001);
        assert_eq!(
            validate_fleet_root_funding_capacity(&changed, [&root]),
            Err(FleetFundingPolicyValidationError::CoordinatorAutomaticCyclesAboveRoots)
        );
    }

    #[test]
    fn preview_multi_subnet_profile_admits_the_bounded_staging_envelope() {
        let root = FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::PreviewMultiSubnet,
                request_threshold: Cycles::new(10 * TRILLION_CYCLES),
                target_balance: Cycles::new(30 * TRILLION_CYCLES),
                cooldown_secs: THIRTY_DAYS_SECS,
                budget: CyclesFundingBudget {
                    window_secs: NINETY_DAYS_SECS,
                    maximum_cycles: Cycles::new(30 * TRILLION_CYCLES),
                },
                maximum_automatic_grants: 2,
                maximum_automatic_cycles: Cycles::new(60 * TRILLION_CYCLES),
            },
            icp_refill: None,
        };
        let coordinator = FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::PreviewMultiSubnet,
            minimum_reserve_cycles: Cycles::new(80 * TRILLION_CYCLES),
            budget: CyclesFundingBudget {
                window_secs: NINETY_DAYS_SECS,
                maximum_cycles: Cycles::new(30 * TRILLION_CYCLES),
            },
            maximum_automatic_grants: 2,
            maximum_automatic_cycles: Cycles::new(60 * TRILLION_CYCLES),
        };

        validate_coordinator_root_funding_policy(&coordinator)
            .expect("bounded preview Coordinator policy");
        validate_fleet_subnet_root_funding_authority(&root, false)
            .expect("bounded preview Root policy");
        validate_fleet_root_funding_admission(&coordinator, &root)
            .expect("preview Coordinator admits the Root target");
        validate_fleet_root_funding_capacity(&coordinator, [&root])
            .expect("preview lifetime caps are mutually bounded");
    }

    fn authority() -> FleetSubnetRootFundingAuthority {
        FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::SingleSubnet,
                request_threshold: Cycles::new(10_000_000_000_000),
                target_balance: Cycles::new(30_000_000_000_000),
                cooldown_secs: THIRTY_DAYS_SECS,
                budget: CyclesFundingBudget {
                    window_secs: NINETY_DAYS_SECS,
                    maximum_cycles: Cycles::new(30_000_000_000_000),
                },
                maximum_automatic_grants: 4,
                maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
            },
            icp_refill: None,
        }
    }

    fn coordinator_policy() -> FleetCoordinatorRootFundingPolicy {
        FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            minimum_reserve_cycles: Cycles::new(30_000_000_000_000),
            budget: CyclesFundingBudget {
                window_secs: NINETY_DAYS_SECS,
                maximum_cycles: Cycles::new(30_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
        }
    }
}
