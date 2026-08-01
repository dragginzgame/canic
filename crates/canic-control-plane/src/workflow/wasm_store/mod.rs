//! Module: workflow::wasm_store
//!
//! Responsibility: orchestrate destructive Store-local retirement effects.
//! Does not own: endpoint authorization, root deletion authority, or stable Store data.
//! Boundary: an authenticated root may reclaim cycles only from its empty GC-complete Store.

use crate::{
    config,
    dto::template::{
        WASM_STORE_DELETION_CALL_REFUND_HEADROOM_CYCLES,
        WASM_STORE_DELETION_MAXIMUM_RETAINED_CYCLES, WasmStoreDeletionCycleReclamationRequest,
        WasmStoreDeletionCycleReclamationResponse,
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
    let target_cycles_to_retain = request
        .maximum_cycles_to_retain
        .saturating_sub(WASM_STORE_DELETION_CALL_REFUND_HEADROOM_CYCLES);
    let maximum_transfer = cycles_before.saturating_sub(target_cycles_to_retain);
    if maximum_transfer == 0 {
        return Ok(reclamation_response(
            destination,
            request.maximum_cycles_to_retain,
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
    let cycles_transferred = cycles_before_transfer.saturating_sub(target_cycles_to_retain);
    let result = transfer_reclaimed_cycles(&permit, destination, cycles_transferred).await;
    settle_cycle_reclamation(&permit, result)?;

    Ok(reclamation_response(
        destination,
        request.maximum_cycles_to_retain,
        cycles_before,
        cycles_transferred,
    ))
}

fn validate_request(
    request: WasmStoreDeletionCycleReclamationRequest,
) -> Result<(), InternalError> {
    if request.maximum_cycles_to_retain <= WASM_STORE_DELETION_CALL_REFUND_HEADROOM_CYCLES
        || request.maximum_cycles_to_retain > WASM_STORE_DELETION_MAXIMUM_RETAINED_CYCLES
    {
        return Err(InternalError::invalid_input(
            "Store deletion cycle reserve is outside the supported range",
        ));
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
        return Err(InternalError::conflict(
            "Store deletion cycles require one empty GC-complete Store",
        ));
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
    maximum_cycles_to_retain: u128,
    cycles_before: u128,
    cycles_transferred: u128,
) -> WasmStoreDeletionCycleReclamationResponse {
    WasmStoreDeletionCycleReclamationResponse {
        destination,
        cycles_before,
        maximum_cycles_to_retain,
        cycles_transferred,
        cycles_after: IcOps::canister_cycle_balance().to_u128(),
    }
}
