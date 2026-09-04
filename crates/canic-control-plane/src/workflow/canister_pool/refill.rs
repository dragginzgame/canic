//! Recoverable IC-mainnet Cycles Ledger refill for the root's prepaid Canister pool.

use crate::{
    ops::canister_pool::{
        CanisterPoolCreationAuthority, CanisterPoolOps, creation_failure_view_to_dto,
    },
    view::canister_pool::{
        CanisterPoolCreationFailureView, CanisterPoolCreationProgressView, CanisterPoolCreationView,
    },
    workflow::deployment,
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::InternalError,
        model::replay::ReplayCostGuardSettlement,
        ops::cost_guard::CostGuardPermit,
        ops::ic::{
            IcOps,
            build_network::BuildNetworkOps,
            cycles_ledger::{
                CyclesLedgerCreateCanisterError, CyclesLedgerCreateCanisterSuccess, CyclesLedgerOps,
            },
        },
        workflow::{
            cost_guard::CostGuardWorkflow, runtime::fleet_activation::FleetActivationWorkflow,
        },
    },
    dto::pool::{CanisterPoolCreationFailure, PoolAdminResponse},
    ids::{BuildNetwork, FleetSubnetCanisterPoolConfig},
};
use sha2::{Digest, Sha256};

const CREATION_OPERATION_DOMAIN: &[u8] = b"canic.root.canister_pool.cycles_ledger_creation.v1";
const FUNDING_RECHECK_INTERVAL_NS: u64 = 60_000_000_000;

pub(super) async fn start(
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<PoolAdminResponse, InternalError> {
    if BuildNetworkOps::build_network() != Some(BuildNetwork::Ic) {
        return Ok(PoolAdminResponse::MaintenancePaused {
            reason: "automatic Canister pool refill requires the IC-mainnet Cycles Ledger"
                .to_string(),
        });
    }
    if CanisterPoolOps::asset_capacity_is_exhausted(config) {
        return Err(InternalError::resource_exhausted());
    }

    let authority = current_creation_authority(config).await?;
    let operation_id =
        creation_operation_id(authority.root, CanisterPoolOps::next_creation_sequence());
    let now_ns = IcOps::now_nanos();
    let created_at_time_ns = CanisterPoolOps::next_creation_timestamp(now_ns)?;
    CanisterPoolOps::begin_creation(
        CanisterPoolCreationAuthority {
            operation_id,
            created_at_time_ns,
            ..authority
        },
        now_ns,
    )?;
    reconcile().await
}

pub(super) async fn reconcile() -> Result<PoolAdminResponse, InternalError> {
    let creation = CanisterPoolOps::pending_creation().ok_or_else(InternalError::unavailable)?;
    validate_creation_authority(&creation).await?;
    match creation.progress {
        CanisterPoolCreationProgressView::Created {
            canister_id,
            block_index: _,
        } => adopt_created(creation, canister_id),
        CanisterPoolCreationProgressView::Blocked { failure } => {
            Ok(PoolAdminResponse::RefillBlocked {
                operation_id: creation.operation_id,
                failure: creation_failure_view_to_dto(failure),
            })
        }
        CanisterPoolCreationProgressView::Intent { uncertain_result } => {
            retry_intent(creation, uncertain_result).await
        }
        CanisterPoolCreationProgressView::WaitingForFunding {
            available_cycles,
            observed_at_ns: _,
            retry_at_ns,
        } => retry_waiting_for_funding(creation, available_cycles, retry_at_ns).await,
    }
}

pub(super) async fn reconcile_draining() -> Result<PoolAdminResponse, InternalError> {
    let creation = CanisterPoolOps::pending_creation().ok_or_else(InternalError::unavailable)?;
    validate_creation_authority(&creation).await?;
    match creation.progress {
        CanisterPoolCreationProgressView::Intent {
            uncertain_result: false,
        }
        | CanisterPoolCreationProgressView::WaitingForFunding { .. }
        | CanisterPoolCreationProgressView::Blocked {
            failure:
                CanisterPoolCreationFailureView::LedgerCreationFailed
                | CanisterPoolCreationFailureView::LedgerRejected,
        } if creation.cost_guard_settlement.is_none() => {
            CanisterPoolOps::cancel_known_unapplied_creation()?;
            Ok(PoolAdminResponse::MaintenancePaused {
                reason: "draining cancelled one known-unapplied Canister pool refill".to_string(),
            })
        }
        CanisterPoolCreationProgressView::Created { canister_id, .. } => {
            adopt_created(creation, canister_id)
        }
        CanisterPoolCreationProgressView::Intent {
            uncertain_result: true,
        } => retry_intent(creation, true).await,
        CanisterPoolCreationProgressView::Blocked { failure } => {
            Ok(PoolAdminResponse::RefillBlocked {
                operation_id: creation.operation_id,
                failure: creation_failure_view_to_dto(failure),
            })
        }
        CanisterPoolCreationProgressView::Intent {
            uncertain_result: false,
        } => Err(InternalError::unavailable()),
        CanisterPoolCreationProgressView::WaitingForFunding { .. } => {
            Err(InternalError::conflict())
        }
    }
}

pub(super) fn retry_blocked() -> Result<PoolAdminResponse, InternalError> {
    let operation_id = CanisterPoolOps::retry_blocked_creation()?;
    Ok(PoolAdminResponse::RefillRetryScheduled {
        previous_operation_id: operation_id,
    })
}

async fn retry_intent(
    creation: CanisterPoolCreationView,
    was_uncertain: bool,
) -> Result<PoolAdminResponse, InternalError> {
    reconcile_previous_cost_guard(&creation, was_uncertain)?;
    let creation = CanisterPoolOps::pending_creation().ok_or_else(InternalError::unavailable)?;
    if !was_uncertain {
        let available = CyclesLedgerOps::balance_of(creation.root).await?.to_u128();
        let required = required_funding(&creation)?;
        if available < required {
            return retain_funding_wait(creation, available, IcOps::now_nanos());
        }
    }
    let permit = deployment::reserve_canister_pool_creation_cost_guard()?;
    let settlement = permit.replay_settlement();
    CanisterPoolOps::begin_creation_attempt(creation.operation_id, settlement, IcOps::now_nanos())
        .map_err(|error| {
            CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
        })?;
    let result = CyclesLedgerOps::create_canister(
        &permit,
        creation.root,
        creation.placement_subnet,
        creation.ledger_amount.clone(),
        creation.created_at_time_ns,
    )
    .await;
    match result {
        Ok(result) => handle_ledger_result(creation, was_uncertain, &permit, result),
        Err(error) => {
            CostGuardWorkflow::recover(&permit, IcOps::now_secs())?;
            CanisterPoolOps::finish_creation_attempt(creation.operation_id, settlement, true)?;
            Err(error)
        }
    }
}

fn handle_ledger_result(
    creation: CanisterPoolCreationView,
    was_uncertain: bool,
    permit: &CostGuardPermit,
    result: Result<CyclesLedgerCreateCanisterSuccess, CyclesLedgerCreateCanisterError>,
) -> Result<PoolAdminResponse, InternalError> {
    let settlement = permit.replay_settlement();
    match result {
        Ok(success) => record_created(
            creation,
            permit,
            CyclesLedgerOps::checked_block_index(success.block_id)?,
            success.canister_id,
        ),
        Err(CyclesLedgerCreateCanisterError::Duplicate {
            duplicate_of,
            canister_id: Some(canister_id),
        }) => record_created(
            creation,
            permit,
            CyclesLedgerOps::checked_block_index(duplicate_of)?,
            canister_id,
        ),
        Err(CyclesLedgerCreateCanisterError::Duplicate {
            canister_id: None, ..
        }) => {
            CostGuardWorkflow::complete(permit, IcOps::now_secs())?;
            CanisterPoolOps::finish_creation_attempt(creation.operation_id, settlement, true)?;
            Ok(PoolAdminResponse::RefillPending {
                operation_id: creation.operation_id,
                uncertain_result: true,
            })
        }
        Err(CyclesLedgerCreateCanisterError::TooOld) => {
            CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
            CanisterPoolOps::finish_creation_attempt(
                creation.operation_id,
                settlement,
                was_uncertain,
            )?;
            handle_expired_creation(creation.operation_id, was_uncertain)
        }
        Err(CyclesLedgerCreateCanisterError::InsufficientFunds { balance }) => {
            CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
            let available = CyclesLedgerOps::checked_cycles(balance)?.to_u128();
            let observed_at_ns = IcOps::now_nanos();
            let retry_at_ns = next_funding_retry(observed_at_ns)?;
            CanisterPoolOps::finish_creation_waiting_for_funding(
                creation.operation_id,
                settlement,
                available,
                observed_at_ns,
                retry_at_ns,
            )?;
            funding_wait_response(&creation, available, retry_at_ns)
        }
        Err(
            CyclesLedgerCreateCanisterError::CreatedInFuture { .. }
            | CyclesLedgerCreateCanisterError::TemporarilyUnavailable,
        ) => {
            CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
            CanisterPoolOps::finish_creation_attempt(creation.operation_id, settlement, false)?;
            Ok(PoolAdminResponse::RefillPending {
                operation_id: creation.operation_id,
                uncertain_result: false,
            })
        }
        Err(CyclesLedgerCreateCanisterError::FailedToCreate { .. }) => block_known_failure(
            creation.operation_id,
            settlement,
            permit,
            CanisterPoolCreationFailure::LedgerCreationFailed,
        ),
        Err(CyclesLedgerCreateCanisterError::GenericError { .. }) => block_known_failure(
            creation.operation_id,
            settlement,
            permit,
            CanisterPoolCreationFailure::LedgerRejected,
        ),
    }
}

fn record_created(
    creation: CanisterPoolCreationView,
    permit: &CostGuardPermit,
    block_index: u64,
    canister_id: Principal,
) -> Result<PoolAdminResponse, InternalError> {
    CanisterPoolOps::mark_creation_created(creation.operation_id, block_index, canister_id)?;
    CanisterPoolOps::register_created_pending_reset(
        creation.operation_id,
        canister_id,
        IcOps::now_nanos(),
    )?;
    CostGuardWorkflow::complete(permit, IcOps::now_secs())?;
    CanisterPoolOps::settle_creation_attempt(creation.operation_id, permit.replay_settlement())?;
    CanisterPoolOps::commit_creation(creation.operation_id)?;
    Ok(PoolAdminResponse::Created { canister_id })
}

fn adopt_created(
    creation: CanisterPoolCreationView,
    canister_id: Principal,
) -> Result<PoolAdminResponse, InternalError> {
    if let Some(settlement) = creation.cost_guard_settlement {
        CostGuardWorkflow::complete_replay_settlement(&settlement, IcOps::now_secs())?;
        CanisterPoolOps::settle_creation_attempt(creation.operation_id, settlement)?;
    }
    CanisterPoolOps::register_created_pending_reset(
        creation.operation_id,
        canister_id,
        IcOps::now_nanos(),
    )?;
    CanisterPoolOps::commit_creation(creation.operation_id)?;
    Ok(PoolAdminResponse::Created { canister_id })
}

fn reconcile_previous_cost_guard(
    creation: &CanisterPoolCreationView,
    uncertain_result: bool,
) -> Result<(), InternalError> {
    let Some(settlement) = creation.cost_guard_settlement else {
        return Ok(());
    };
    CostGuardWorkflow::recover_replay_settlement(&settlement, IcOps::now_secs())?;
    CanisterPoolOps::finish_creation_attempt(creation.operation_id, settlement, uncertain_result)
}

fn block_known_failure(
    operation_id: [u8; 32],
    settlement: ReplayCostGuardSettlement,
    permit: &CostGuardPermit,
    failure: CanisterPoolCreationFailure,
) -> Result<PoolAdminResponse, InternalError> {
    CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
    CanisterPoolOps::finish_creation_attempt(operation_id, settlement, false)?;
    CanisterPoolOps::block_creation(operation_id, failure)?;
    Ok(PoolAdminResponse::RefillBlocked {
        operation_id,
        failure,
    })
}

fn handle_expired_creation(
    operation_id: [u8; 32],
    uncertain_result: bool,
) -> Result<PoolAdminResponse, InternalError> {
    if uncertain_result {
        let failure = CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow;
        CanisterPoolOps::block_creation(operation_id, failure)?;
        return Ok(PoolAdminResponse::RefillBlocked {
            operation_id,
            failure,
        });
    }
    CanisterPoolOps::rollover_known_expired_creation()?;
    Ok(PoolAdminResponse::RefillRetryScheduled {
        previous_operation_id: operation_id,
    })
}

async fn validate_creation_authority(
    creation: &CanisterPoolCreationView,
) -> Result<(), InternalError> {
    let binding = FleetActivationWorkflow::root_authority()?.binding;
    let expected = current_creation_authority(&binding.limits.canister_pool).await?;
    let actual = CanisterPoolCreationAuthority {
        creation_execution_margin: creation.creation_execution_margin.clone(),
        operation_id: [0; 32],
        cycles_ledger: creation.cycles_ledger,
        placement_subnet: creation.placement_subnet,
        root: creation.root,
        ledger_amount: creation.ledger_amount.clone(),
        ledger_fee: creation.ledger_fee.clone(),
        management_creation_fee: creation.management_creation_fee.clone(),
        readiness_floor: creation.readiness_floor.clone(),
        created_at_time_ns: 0,
    };
    if actual != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

async fn current_creation_authority(
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<CanisterPoolCreationAuthority, InternalError> {
    let binding = FleetActivationWorkflow::root_authority()?.binding;
    let root = binding.fleet_subnet_root;
    let funded_native_cycles = config
        .canister_cycles
        .to_u128()
        .checked_add(config.creation_execution_margin.to_u128())
        .map(Cycles::new)
        .ok_or_else(InternalError::resource_exhausted)?;
    let ledger_amount = IcOps::canister_creation_attached_cycles(&funded_native_cycles)?;
    let management_creation_fee = ledger_amount
        .to_u128()
        .checked_sub(funded_native_cycles.to_u128())
        .map(Cycles::new)
        .ok_or_else(InternalError::conflict)?;
    Ok(CanisterPoolCreationAuthority {
        creation_execution_margin: config.creation_execution_margin.clone(),
        operation_id: [0; 32],
        cycles_ledger: CyclesLedgerOps::canister_id(),
        placement_subnet: binding.placement_subnet.into_principal(),
        root,
        ledger_amount,
        ledger_fee: CyclesLedgerOps::fee().await?,
        management_creation_fee,
        readiness_floor: config.canister_cycles.clone(),
        created_at_time_ns: 0,
    })
}

async fn retry_waiting_for_funding(
    creation: CanisterPoolCreationView,
    retained_available: u128,
    retry_at_ns: u64,
) -> Result<PoolAdminResponse, InternalError> {
    let now_ns = IcOps::now_nanos();
    if now_ns < retry_at_ns {
        return funding_wait_response(&creation, retained_available, retry_at_ns);
    }
    let available = CyclesLedgerOps::balance_of(creation.root).await?.to_u128();
    let required = required_funding(&creation)?;
    if available < required {
        return retain_funding_wait(creation, available, now_ns);
    }
    CanisterPoolOps::resume_creation_after_funding(creation.operation_id, available)?;
    let resumed = CanisterPoolOps::pending_creation().ok_or_else(InternalError::unavailable)?;
    retry_intent(resumed, false).await
}

fn retain_funding_wait(
    creation: CanisterPoolCreationView,
    available: u128,
    observed_at_ns: u64,
) -> Result<PoolAdminResponse, InternalError> {
    let retry_at_ns = next_funding_retry(observed_at_ns)?;
    CanisterPoolOps::wait_for_creation_funding(
        creation.operation_id,
        available,
        observed_at_ns,
        retry_at_ns,
    )?;
    funding_wait_response(&creation, available, retry_at_ns)
}

fn funding_wait_response(
    creation: &CanisterPoolCreationView,
    available: u128,
    retry_at_ns: u64,
) -> Result<PoolAdminResponse, InternalError> {
    let required = required_funding(creation)?;
    Ok(PoolAdminResponse::RefillWaitingForCycles {
        available: Cycles::new(available),
        attempt_count: creation.attempt_count,
        creation_amount: creation.ledger_amount.clone(),
        execution_margin: creation.creation_execution_margin.clone(),
        last_attempt_at_ns: creation.last_attempt_at_ns,
        ledger_fee: creation.ledger_fee.clone(),
        readiness_floor: creation.readiness_floor.clone(),
        required: Cycles::new(required),
        retry_at_ns,
        shortfall: Cycles::new(required.saturating_sub(available)),
    })
}

fn required_funding(creation: &CanisterPoolCreationView) -> Result<u128, InternalError> {
    creation
        .ledger_amount
        .to_u128()
        .checked_add(creation.ledger_fee.to_u128())
        .ok_or_else(InternalError::resource_exhausted)
}

fn next_funding_retry(now_ns: u64) -> Result<u64, InternalError> {
    now_ns
        .checked_add(FUNDING_RECHECK_INTERVAL_NS)
        .ok_or_else(InternalError::resource_exhausted)
}

fn creation_operation_id(root: Principal, sequence: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(CREATION_OPERATION_DOMAIN);
    hasher.update(root.as_slice());
    hasher.update(sequence.to_be_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_identity_binds_root_and_never_reuses_a_sequence() {
        let root = Principal::from_slice(&[1; 29]);
        assert_eq!(
            creation_operation_id(root, 7),
            creation_operation_id(root, 7)
        );
        assert_ne!(
            creation_operation_id(root, 7),
            creation_operation_id(root, 8)
        );
        assert_ne!(
            creation_operation_id(root, 7),
            creation_operation_id(Principal::from_slice(&[2; 29]), 7)
        );
    }

    #[test]
    fn unresolved_expiry_is_never_operator_retryable() {
        CanisterPoolOps::clear_for_test();
        let root = Principal::from_slice(&[1; 29]);
        let operation_id = creation_operation_id(root, 0);
        CanisterPoolOps::begin_creation(
            CanisterPoolCreationAuthority {
                creation_execution_margin: Cycles::new(1),
                operation_id,
                cycles_ledger: Principal::from_slice(&[2; 29]),
                placement_subnet: Principal::from_slice(&[3; 29]),
                root,
                ledger_amount: Cycles::new(10),
                ledger_fee: Cycles::new(1),
                management_creation_fee: Cycles::new(1),
                readiness_floor: Cycles::new(8),
                created_at_time_ns: 4,
            },
            4,
        )
        .expect("begin creation");
        CanisterPoolOps::block_creation(
            operation_id,
            CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow,
        )
        .expect("block unresolved creation");
        assert!(CanisterPoolOps::retry_blocked_creation().is_err());
        CanisterPoolOps::clear_for_test();
    }
}
