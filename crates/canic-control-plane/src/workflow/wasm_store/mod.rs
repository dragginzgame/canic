//! Module: workflow::wasm_store
//!
//! Responsibility: orchestrate destructive Store-local retirement effects.
//! Does not own: endpoint authorization, root deletion authority, or stable Store data.
//! Boundary: an authenticated root may reclaim cycles only from its empty GC-complete Store.

use crate::{
    config,
    dto::template::{
        WasmStoreDeletionCycleReclamationRequest, WasmStoreDeletionCycleReclamationResponse,
    },
    ids::{WasmStoreGcMode, WasmStoreGcStatus},
    ops::storage::template::{
        TemplateChunkedOps, TemplateManifestOps, WasmStoreGcOps, WasmStoreLimits,
    },
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            cost_guard::{CostGuardPermit, CostGuardRequest},
            ic::{IcOps, mgmt::MgmtOps},
        },
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
    },
    replay_policy::CostClass,
};

const STORE_DELETION_CYCLE_RECLAMATION_COMMAND_KIND: &str = "wasm_store.reclaim_deletion_cycles.v1";
const VALUE_TRANSFER_QUOTA_WINDOW_SECONDS: u64 = 60;
const MAX_VALUE_TRANSFERS_PER_WINDOW: u64 = 60;

/// Return every transferable cycle above the root-calculated deletion reserve.
pub async fn reclaim_deletion_cycles(
    request: WasmStoreDeletionCycleReclamationRequest,
) -> Result<WasmStoreDeletionCycleReclamationResponse, InternalError> {
    validate_request(request)?;
    require_empty_gc_complete_store()?;

    let destination = IcOps::msg_caller();
    let cycles_before = IcOps::canister_cycle_balance().to_u128();
    let deposit_call_cost = MgmtOps::deposit_cycles_call_cost(destination)?;
    let target_cycles_to_retain = request
        .retained_cycles_target
        .checked_sub(deposit_call_cost)
        .ok_or_else(|| InternalError::invalid_input())?;
    let maximum_transfer =
        transferable_cycles(cycles_before, target_cycles_to_retain, deposit_call_cost);
    if maximum_transfer == 0 {
        return Ok(reclamation_response(
            destination,
            request.retained_cycles_target,
            cycles_before,
            0,
        ));
    }

    let permit = reserve_cycle_reclamation(
        destination,
        maximum_transfer,
        target_cycles_to_retain,
        cycles_before,
    )?;
    let cycles_before_transfer = IcOps::canister_cycle_balance().to_u128();
    let cycles_transferred = transferable_cycles(
        cycles_before_transfer,
        target_cycles_to_retain,
        deposit_call_cost,
    );
    let result = transfer_reclaimed_cycles(&permit, destination, cycles_transferred).await;
    settle_cycle_reclamation(&permit, result)?;

    Ok(reclamation_response(
        destination,
        request.retained_cycles_target,
        cycles_before,
        cycles_transferred,
    ))
}

const fn transferable_cycles(
    current_cycles: u128,
    target_cycles_to_retain: u128,
    call_cost: u128,
) -> u128 {
    current_cycles
        .saturating_sub(target_cycles_to_retain)
        .saturating_sub(call_cost)
}

fn validate_request(
    request: WasmStoreDeletionCycleReclamationRequest,
) -> Result<(), InternalError> {
    if request.retained_cycles_target == 0 {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn require_empty_gc_complete_store() -> Result<(), InternalError> {
    let gc = WasmStoreGcOps::snapshot();
    let status = local_store_status(gc)?;
    let is_empty_and_terminal = [
        status.gc.mode == WasmStoreGcMode::Complete,
        status.gc.runs_completed == 1,
        status.occupied_store_bytes == 0,
        status.template_count == 0,
        status.release_count == 0,
        status.templates.is_empty(),
        TemplateManifestOps::approved_catalog_response().is_empty(),
    ]
    .into_iter()
    .all(|valid| valid);
    if !is_empty_and_terminal {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn local_store_status(
    gc: WasmStoreGcStatus,
) -> Result<crate::dto::template::WasmStoreStatusResponse, InternalError> {
    let store = config::current_wasm_store()?;
    let limits = WasmStoreLimits::from(&store);
    Ok(TemplateChunkedOps::store_status_response(
        limits,
        store.headroom_bytes(),
        gc,
    ))
}

fn reserve_cycle_reclamation(
    destination: Principal,
    maximum_transfer: u128,
    retained_cycles: u128,
    current_cycle_balance: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardWorkflow::reserve(CostGuardRequest {
        cost_class: CostClass::ValueTransfer,
        command_kind: CommandKind::new(STORE_DELETION_CYCLE_RECLAMATION_COMMAND_KIND)
            .expect("Store deletion cycle-reclamation command kind is valid"),
        quota_subject: destination,
        payer: IcOps::canister_self(),
        now_secs: IcOps::now_secs(),
        quota_window_secs: VALUE_TRANSFER_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_VALUE_TRANSFERS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: maximum_transfer,
        min_cycles_after_reservation: retained_cycles,
    })
    .map_err(map_cost_guard_reserve_error)
}

async fn transfer_reclaimed_cycles(
    permit: &CostGuardPermit,
    destination: Principal,
    cycles: u128,
) -> Result<(), InternalError> {
    if cycles == 0 {
        return Ok(());
    }
    MgmtOps::deposit_cycles_with_permit(permit, destination, cycles).await
}

fn settle_cycle_reclamation(
    permit: &CostGuardPermit,
    result: Result<(), InternalError>,
) -> Result<(), InternalError> {
    match result {
        Ok(()) => CostGuardWorkflow::complete(permit, IcOps::now_secs()),
        Err(error) => Err(CostGuardWorkflow::recover_after_failure(
            permit,
            IcOps::now_secs(),
            error,
        )),
    }
}

fn reclamation_response(
    destination: Principal,
    retained_cycles_target: u128,
    cycles_before: u128,
    cycles_transferred: u128,
) -> WasmStoreDeletionCycleReclamationResponse {
    WasmStoreDeletionCycleReclamationResponse {
        destination,
        cycles_before,
        retained_cycles_target,
        cycles_transferred,
        cycles_after: IcOps::canister_cycle_balance().to_u128(),
    }
}

#[cfg(test)]
mod tests {
    use super::transferable_cycles;

    #[test]
    fn cycle_reclamation_retains_the_target_and_exact_call_cost() {
        let retained_cycles_target = 200_u128;
        let call_cost = 60;
        let target_before_call = retained_cycles_target.saturating_sub(call_cost);

        assert_eq!(
            transferable_cycles(1_500, target_before_call, call_cost),
            1_300
        );
        assert_eq!(transferable_cycles(190, target_before_call, call_cost), 0);
    }
}
