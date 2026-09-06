//! Module: workflow::fleet_coordinator::retirement
//!
//! Responsibility: return the removed Fleet's Ledger balance to its authenticated operator.
//! Does not own: Ledger identity construction, stable mutation or physical canister deletion.
//! Boundary: exact durable transfer identity and verified empty source precede completion.

use crate::ops::fleet_coordinator::FleetCoordinatorOps;
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            cost_guard::CostGuardRequest,
            ic::{
                IcOps,
                cycles_ledger::{CyclesLedgerOps, CyclesLedgerTransferError},
            },
        },
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
    },
    dto::fleet_registry::{FleetRetirementRequest, FleetRetirementStatus},
    replay_policy::CostClass,
};

pub async fn retire(
    caller: Principal,
    request: FleetRetirementRequest,
) -> Result<FleetRetirementStatus, InternalError> {
    let status = FleetCoordinatorOps::retirement_preflight(caller, request, IcOps::now_nanos())?;
    if status.ledger_receipt.is_some() {
        return Ok(status);
    }
    let transfer = if let Some(transfer) = status.ledger_transfer {
        transfer
    } else {
        let balance = CyclesLedgerOps::balance_of(IcOps::canister_self())
            .await?
            .to_u128();
        let fee = if balance == 0 {
            0
        } else {
            CyclesLedgerOps::fee().await?.to_u128()
        };
        FleetCoordinatorOps::begin_retirement_transfer(
            caller,
            status.request,
            balance,
            fee,
            IcOps::now_nanos(),
        )?
    };
    let block_index = if transfer.balance_before == 0 {
        None
    } else {
        let permit = CostGuardWorkflow::reserve(CostGuardRequest {
            cost_class: CostClass::ValueTransfer,
            command_kind: CommandKind::new("fleet.retirement.ledger.transfer.v1")
                .expect("constant command identity"),
            quota_subject: caller,
            payer: IcOps::canister_self(),
            now_secs: IcOps::now_secs(),
            quota_window_secs: 60,
            max_operations_per_window: 60,
            current_cycle_balance: IcOps::canister_cycle_balance().to_u128(),
            cycle_reservation_cycles: 0,
            min_cycles_after_reservation: 0,
        })
        .map_err(map_cost_guard_reserve_error)?;
        let response = CyclesLedgerOps::transfer(&permit, &transfer)
            .await
            .map_err(|error| {
                CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
            })?;
        CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
        let block = match response {
            Ok(block)
            | Err(CyclesLedgerTransferError::Duplicate {
                duplicate_of: block,
            }) => block,
            Err(CyclesLedgerTransferError::TemporarilyUnavailable) => {
                return Err(InternalError::unavailable());
            }
            Err(_) => return Err(InternalError::conflict()),
        };
        Some(u128::try_from(block.0).map_err(|_| InternalError::invariant())?)
    };
    if CyclesLedgerOps::balance_of(IcOps::canister_self())
        .await?
        .to_u128()
        != 0
    {
        return Err(InternalError::conflict());
    }
    FleetCoordinatorOps::complete_retirement_transfer(transfer.memo, block_index)
}
