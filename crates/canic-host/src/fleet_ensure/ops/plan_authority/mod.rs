//! Module: fleet_ensure::ops::plan_authority
//!
//! Responsibility: compare the exact retained plan authority while excluding moving estimates.
//! Does not own: plan hashing, spending approval, balance bounds, or record mutation.
//! Boundary: borrowed projections preserve exact action authority and ordered membership.

use crate::fleet_ensure::model::{
    CanisterDisposition, CanisterPlan, CycleConservation, EnsureAction, EstateFundingDomainPlan,
    EstatePoolPendingCreationObservation, FleetEnsureContinuationAuthority, FleetEnsurePlan,
    FleetEnsurePlanScope, FleetRecoveryReview, FleetReinstallRecord, PoolFundingAuthority,
    PoolRecoveryFunding, RecoveryDiscovery, RetainedRootStartAuthorityRecord,
    RootManagementBinding, RootStartupFundingForecast, infrastructure_bootstrap,
};

/// Compare immutable decisions after callers independently check balance movement and funding.
pub(in crate::fleet_ensure) fn same_plan_authority(
    retained: &FleetEnsurePlan,
    current: &FleetEnsurePlan,
) -> bool {
    let identity_matches = PlanAuthority::from(retained) == PlanAuthority::from(current);
    let canisters_match = same_slice(&retained.canisters, &current.canisters, same_canister);
    let domains_match = same_slice(
        estate_domains(&retained.conservation),
        estate_domains(&current.conservation),
        |left, right| EstateDomainAuthority::from(left) == EstateDomainAuthority::from(right),
    );
    let review_matches = same_recovery_review(
        retained.recovery_review.as_deref(),
        current.recovery_review.as_deref(),
    );
    identity_matches && canisters_match && domains_match && review_matches
}

fn same_slice<T>(left: &[T], right: &[T], matches: impl Fn(&T, &T) -> bool) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| matches(left, right))
}

fn same_canister(left: &CanisterPlan, right: &CanisterPlan) -> bool {
    CanisterAuthority::from(left) == CanisterAuthority::from(right)
        && same_slice(&left.actions, &right.actions, |left, right| {
            CanisterActionAuthority::from(left) == CanisterActionAuthority::from(right)
        })
}

fn same_recovery_review(
    left: Option<&FleetRecoveryReview>,
    right: Option<&FleetRecoveryReview>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            RecoveryAuthority::from(left) == RecoveryAuthority::from(right)
                && same_slice(
                    &left.known_pool_funding,
                    &right.known_pool_funding,
                    |left, right| {
                        RecoveryFundingAuthority::from(left)
                            == RecoveryFundingAuthority::from(right)
                    },
                )
        }
        _ => false,
    }
}

// Exhaustive destructuring requires every new persisted field to be classified.
#[derive(Eq, PartialEq)]
struct PlanAuthority<'a> {
    infrastructure_bootstrap:
        &'a Option<Box<infrastructure_bootstrap::InfrastructureBootstrapRecord>>,
    reinstall: &'a Option<Box<FleetReinstallRecord>>,
    continuation: &'a Option<FleetEnsureContinuationAuthority>,
    desired_sha256: &'a String,
    environment: &'a String,
    fleet: &'a String,
    operation_id: &'a String,
    planned_at_time: &'a u64,
    protocol_actions: &'a Vec<EnsureAction>,
    root_start_authority: &'a Option<Box<RetainedRootStartAuthorityRecord>>,
    root_reinstall_bindings: &'a Vec<RootManagementBinding>,
    schema_version: &'a u16,
    scope: &'a FleetEnsurePlanScope,
    terminal_inventory_operation_id: &'a Option<String>,
}

impl<'a> From<&'a FleetEnsurePlan> for PlanAuthority<'a> {
    fn from(value: &'a FleetEnsurePlan) -> Self {
        let FleetEnsurePlan {
            infrastructure_bootstrap,
            reinstall,
            continuation,
            desired_sha256,
            environment,
            fleet,
            operation_id,
            planned_at_time,
            protocol_actions,
            root_start_authority,
            root_reinstall_bindings,
            schema_version,
            scope,
            terminal_inventory_operation_id,
            canisters: _,
            conservation: _,
            recovery_review: _,
            plan_sha256: _,
            reviewed_desired: _,
        } = value;
        Self {
            infrastructure_bootstrap,
            reinstall,
            continuation,
            desired_sha256,
            environment,
            fleet,
            operation_id,
            planned_at_time,
            protocol_actions,
            root_start_authority,
            root_reinstall_bindings,
            schema_version,
            scope,
            terminal_inventory_operation_id,
        }
    }
}

#[derive(Eq, PartialEq)]
struct CanisterAuthority<'a> {
    disposition: &'a CanisterDisposition,
    name: &'a String,
    principal: &'a Option<String>,
}

impl<'a> From<&'a CanisterPlan> for CanisterAuthority<'a> {
    fn from(value: &'a CanisterPlan) -> Self {
        let CanisterPlan {
            disposition,
            name,
            principal,
            actions: _,
            observed_cycles: _,
        } = value;
        Self {
            disposition,
            name,
            principal,
        }
    }
}

#[derive(Eq, PartialEq)]
enum CanisterActionAuthority<'a> {
    Funding {
        pool_funding: &'a Option<PoolFundingAuthority>,
        created_at_time: &'a u64,
        expected_post_cycles: &'a u128,
        funding_margin_cycles: &'a u128,
        ledger: &'a String,
        name: &'a String,
        principal: &'a String,
    },
    Exact(&'a EnsureAction),
}

impl<'a> From<&'a EnsureAction> for CanisterActionAuthority<'a> {
    fn from(action: &'a EnsureAction) -> Self {
        match action {
            EnsureAction::Fund {
                pool_funding,
                amount: _,
                created_at_time,
                expected_post_cycles,
                funding_deficit_cycles: _,
                funding_margin_cycles,
                ledger,
                name,
                principal,
            } => Self::Funding {
                pool_funding,
                created_at_time,
                expected_post_cycles,
                funding_margin_cycles,
                ledger,
                name,
                principal,
            },
            _ => Self::Exact(action),
        }
    }
}

fn estate_domains(conservation: &CycleConservation) -> &[EstateFundingDomainPlan] {
    let CycleConservation {
        estate_funding_domains,
        expected_post_operation_cycles: _,
        maximum_execution_burn_cycles: _,
        maximum_new_funding_cycles: _,
        maximum_operator_debit_cycles: _,
        maximum_unavoidable_fee_cycles: _,
        observed_controlled_cycles: _,
        retained_in_reused_canisters_cycles: _,
        scheduled_transfer_cycles: _,
    } = conservation;
    estate_funding_domains
}

#[derive(Eq, PartialEq)]
struct EstateDomainAuthority<'a> {
    allocated_workloads: &'a u32,
    available_pool_slots: &'a u32,
    creation_amount_cycles: &'a u128,
    cycles_ledger: &'a String,
    creation_execution_margin_cycles: &'a u128,
    readiness_floor_cycles: &'a u128,
    eligible_ready_pool_assets: &'a u32,
    initial_pool_assets: &'a Vec<String>,
    ledger_fee_cycles: &'a u128,
    management_creation_fee_cycles: &'a u128,
    maximum_creation_debit_cycles: &'a u128,
    maximum_creation_fee_cycles: &'a u128,
    occupied_pool_assets: &'a u32,
    pending_creation_count: &'a u32,
    pending_creation: &'a Option<EstatePoolPendingCreationObservation>,
    pool_maximum_size: &'a u32,
    planned_initial_workloads: &'a u32,
    required_creation_count: &'a u32,
    root: &'a String,
}

impl<'a> From<&'a EstateFundingDomainPlan> for EstateDomainAuthority<'a> {
    fn from(value: &'a EstateFundingDomainPlan) -> Self {
        let EstateFundingDomainPlan {
            allocated_workloads,
            available_pool_slots,
            creation_amount_cycles,
            cycles_ledger,
            creation_execution_margin_cycles,
            readiness_floor_cycles,
            eligible_ready_pool_assets,
            initial_pool_assets,
            ledger_fee_cycles,
            management_creation_fee_cycles,
            maximum_creation_debit_cycles,
            maximum_creation_fee_cycles,
            occupied_pool_assets,
            pending_creation_count,
            pending_creation,
            pool_maximum_size,
            planned_initial_workloads,
            required_creation_count,
            root,
            available_cycles: _,
            maximum_funding_cycles: _,
            root_principal: _,
            shortfall_cycles: _,
        } = value;
        Self {
            allocated_workloads,
            available_pool_slots,
            creation_amount_cycles,
            cycles_ledger,
            creation_execution_margin_cycles,
            readiness_floor_cycles,
            eligible_ready_pool_assets,
            initial_pool_assets,
            ledger_fee_cycles,
            management_creation_fee_cycles,
            maximum_creation_debit_cycles,
            maximum_creation_fee_cycles,
            occupied_pool_assets,
            pending_creation_count,
            pending_creation,
            pool_maximum_size,
            planned_initial_workloads,
            required_creation_count,
            root,
        }
    }
}

#[derive(Eq, PartialEq)]
struct RecoveryAuthority<'a> {
    base_execution_burn_cycles: &'a u128,
    whole_continuation_ceiling_cycles: &'a u128,
    maximum_successor_actions: &'a u32,
    fixture_publication_retry_attempts: &'a u32,
    per_step_burn_cycles: &'a u128,
    startup_funding: &'a Vec<RootStartupFundingForecast>,
    discovery: &'a RecoveryDiscovery,
}

impl<'a> From<&'a FleetRecoveryReview> for RecoveryAuthority<'a> {
    fn from(value: &'a FleetRecoveryReview) -> Self {
        let FleetRecoveryReview {
            base_execution_burn_cycles,
            whole_continuation_ceiling_cycles,
            maximum_successor_actions,
            fixture_publication_retry_attempts,
            per_step_burn_cycles,
            startup_funding,
            discovery,
            continuation_reserve_cycles: _,
            known_pool_funding: _,
        } = value;
        Self {
            base_execution_burn_cycles,
            whole_continuation_ceiling_cycles,
            maximum_successor_actions,
            fixture_publication_retry_attempts,
            per_step_burn_cycles,
            startup_funding,
            discovery,
        }
    }
}

#[derive(Eq, PartialEq)]
struct RecoveryFundingAuthority<'a> {
    principal: &'a String,
    root: &'a String,
    ledger_fee_cycles: &'a u128,
    funding_margin_cycles: &'a u128,
    expected_post_cycles: &'a u128,
}

impl<'a> From<&'a PoolRecoveryFunding> for RecoveryFundingAuthority<'a> {
    fn from(value: &'a PoolRecoveryFunding) -> Self {
        let PoolRecoveryFunding {
            principal,
            root,
            ledger_fee_cycles,
            funding_margin_cycles,
            expected_post_cycles,
            amount_cycles: _,
            funding_deficit_cycles: _,
        } = value;
        Self {
            principal,
            root,
            ledger_fee_cycles,
            funding_margin_cycles,
            expected_post_cycles,
        }
    }
}
