//! Durable Root import reservation, exclusive pool ownership and exact reset receipts.
//!
//! Authenticated workflow supplies verified destination authority and coherent observations.
//! Observations use the existing management adapter; issued reset effects are never blindly retried.

pub mod bootstrap;
mod history;
#[cfg(test)]
mod tests;

use crate::{
    ops::{
        canister_pool::CanisterPoolOps, component_provisioning::RootComponentProvisioningOps,
        component_registry::ComponentRegistryOps,
    },
    storage::{
        stable::canister_pool::{
            CANISTER_POOL_STATE_MAX_BYTES, CanisterPoolAssetOriginRecord, CanisterPoolAssetRecord,
            CanisterPoolAssetStatusRecord, CanisterPoolStateRecord, CanisterPoolStore,
            capacity_import::{
                PoolImportPhaseRecord, PoolImportRecord, PoolImportResetProgressRecord,
            },
        },
        transient::capacity_import::PoolImportExecutionGuard,
    },
    view::canister_pool::{
        PoolImportCallBudgetView, PoolImportHistoryKind, PoolImportHistoryView,
        PoolImportObservationView,
    },
};
use canic_core::{
    cdk::{
        structures::Storable,
        types::{Cycles, Principal},
    },
    control_plane_support::error::InternalError,
    dto::pool_import::{
        PoolImportCommand, PoolImportIdentity, PoolImportPhase, PoolImportReservation,
        PoolImportRootReceipt, PoolImportSource, PoolImportSourceProgress, PoolImportSourceReceipt,
        PoolImportStatus,
    },
    ids::{FleetSubnetCanisterPoolConfig, MAX_FLEET_CAPACITY_IMPORT_SOURCES},
};
use sha2::{Digest, Sha256};

/// Exact reservation and once-only reset owner within the existing pool state cell.
pub struct CanisterPoolImportOps;

impl CanisterPoolImportOps {
    /// Serialize paid transitions without discarding durable effect or debit evidence.
    pub fn claim_execution(
        identity: PoolImportIdentity,
    ) -> Result<PoolImportExecutionGuard, InternalError> {
        Self::status(identity)?;
        PoolImportExecutionGuard::try_claim().ok_or_else(InternalError::unavailable)
    }

    /// Return reviewed caller authority without mutating reservation or observing the IC.
    pub fn operator(command: &PoolImportCommand) -> Result<Principal, InternalError> {
        let identity = match command {
            PoolImportCommand::Reserve(request) => return Ok(request.operator),
            PoolImportCommand::Advance { identity, .. }
            | PoolImportCommand::Release { identity, .. }
            | PoolImportCommand::Settle(identity) => *identity,
        };
        Ok(Self::status(identity)?.reservation.operator)
    }

    /// Bind the complete current Root authority, including controller and pool policy.
    pub fn authority_hash(
        binding: &canic_core::ids::FleetSubnetRootBinding,
    ) -> Result<[u8; 32], InternalError> {
        let bytes = candid::encode_one(binding).map_err(|_| InternalError::invariant())?;
        let mut hash = Sha256::new();
        hash.update(b"canic:capacity-import:root-authority:v1\0");
        hash.update(bytes);
        Ok(hash.finalize().into())
    }

    /// Project one coherent management response, retaining its exact version.
    pub async fn observe_source(
        canister_id: Principal,
    ) -> Result<PoolImportObservationView, InternalError> {
        let observed =
            canic_core::control_plane_support::ops::ic::mgmt::MgmtOps::canister_status(canister_id)
                .await?;
        if observed.status
            == canic_core::control_plane_support::ops::ic::mgmt::CanisterStatusType::Stopping
        {
            return Err(InternalError::conflict());
        }
        let mut controllers = observed.settings.controllers;
        controllers.sort_unstable();
        let module_sha256 = observed
            .module_hash
            .map(|hash| hash.try_into().map_err(|_| InternalError::invariant()))
            .transpose()?;
        Ok(PoolImportObservationView {
            canister_id,
            canister_version: observed.version,
            stopped: observed.status
                == canic_core::control_plane_support::ops::ic::mgmt::CanisterStatusType::Stopped,
            snapshots_size_bytes: observed
                .memory_metrics
                .snapshots_size
                .0
                .try_into()
                .map_err(|_| InternalError::invariant())?,
            module_sha256,
            controllers,
            reserved_cycles: Cycles::try_from(observed.reserved_cycles)
                .map_err(|_| InternalError::invariant())?
                .to_u128(),
            cycles: Cycles::try_from(observed.cycles)
                .map_err(|_| InternalError::invariant())?
                .to_u128(),
        })
    }

    /// Observe exact replicated management history through the same metric-owning adapter.
    pub async fn observe_history(
        canister_id: Principal,
    ) -> Result<PoolImportHistoryView, InternalError> {
        let response = canic_core::control_plane_support::ops::ic::mgmt::MgmtOps::canister_history(
            canister_id,
        )
        .await?;
        if response.canister_id != canister_id {
            return Err(InternalError::conflict());
        }
        history::decode(canister_id, &response.history_candid)
    }

    /// Current monotonic reservation number; a consumed identity can never start again.
    #[must_use]
    pub fn next_sequence() -> u64 {
        CanisterPoolStore::state().next_import_sequence
    }

    /// Identify the current exclusive owner for review diagnostics without paid observations.
    pub fn active_identity() -> Option<PoolImportIdentity> {
        CanisterPoolStore::state()
            .capacity_import
            .filter(|record| !matches!(record.phase, PoolImportPhaseRecord::Released { .. }))
            .map(|record| PoolImportIdentity {
                sequence: record.reservation.sequence,
                plan_sha256: record.reservation.plan_sha256,
            })
    }

    /// Ordinary pool effects must be fenced for the entire unfinished import.
    pub fn require_idle() -> Result<(), InternalError> {
        if Self::is_active() {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    #[must_use]
    pub fn is_active() -> bool {
        bootstrap::pending() || Self::active_identity().is_some()
    }

    /// Reject competing authority/provisioning work before review or reservation.
    pub fn require_no_competing_operation() -> Result<(), InternalError> {
        ComponentRegistryOps::require_admission_catalog_stable()?;
        let provisioning = RootComponentProvisioningOps::active_operation()?.is_some();
        let draining = ComponentRegistryOps::current()
            .is_some_and(|registry| registry.root_draining.is_some());
        if provisioning || draining {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    /// Reserve all physical IDs atomically before the operator grants Root control.
    /// Workflow must authenticate the caller, current Root authority and source subnet.
    pub fn reserve(
        reservation: PoolImportReservation,
        config: &FleetSubnetCanisterPoolConfig,
        now_ns: u64,
    ) -> Result<PoolImportStatus, InternalError> {
        validate_reservation(&reservation, config)?;
        let mut state = CanisterPoolStore::state();
        if let Some(record) = &state.capacity_import {
            if record.reservation == reservation {
                return Ok(status(record));
            }
            if !matches!(record.phase, PoolImportPhaseRecord::Released { .. }) {
                return Err(InternalError::conflict());
            }
        }
        if reservation.sequence != state.next_import_sequence {
            return Err(InternalError::conflict());
        }
        let next_sequence = state
            .next_import_sequence
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        bootstrap::require_reservation(&state, &reservation)?;
        require_quiet_pool(&state)?;
        Self::require_no_competing_operation()?;
        for source in &reservation.sources {
            if CanisterPoolStore::get(&source.canister_id).is_some()
                || ComponentRegistryOps::component_for_principal(source.canister_id).is_some()
            {
                return Err(InternalError::conflict());
            }
        }
        let ids = reservation
            .sources
            .iter()
            .map(|source| source.canister_id)
            .collect::<Vec<_>>();
        let record = PoolImportRecord {
            reserved_at_ns: now_ns,
            root_receipt: None,
            progress: vec![PoolImportResetProgressRecord::AwaitingHandoff; ids.len()],
            phase: PoolImportPhaseRecord::Reserved,
            paid_calls: 0,
            reserved_debit_cycles: 0,
            last_root_cycles: reservation.observed_root_cycles,
            reservation,
        };
        state.next_import_sequence = next_sequence;
        state.capacity_import = Some(record.clone());
        require_completion_fits(&state)?;
        // IDs reserve capacity here; they enter physical inventory only after
        // a management observation proves Root actually controls each source.
        crate::ops::canister_pool::validate_config(config)?;
        let count = u64::try_from(ids.len()).map_err(|_| InternalError::resource_exhausted())?;
        if CanisterPoolOps::occupied_asset_capacity()
            .checked_add(count)
            .is_none_or(|occupied| occupied > u64::from(config.maximum_size))
        {
            return Err(InternalError::resource_exhausted());
        }
        CanisterPoolStore::set_state(state);
        Ok(status(&record))
    }

    /// Read protected retained evidence without observing or touching the physical source.
    pub fn status(identity: PoolImportIdentity) -> Result<PoolImportStatus, InternalError> {
        let state = CanisterPoolStore::state();
        Ok(status(required(&state, identity)?))
    }

    /// Charge a conservative call bound before any paid request, including reconciliation.
    /// Failed calls keep their reservation; restart never restores the spent allowance.
    pub fn reserve_paid_call(
        identity: PoolImportIdentity,
        maximum_call_debit: u128,
        observed_root_cycles: u128,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let record = required_mut(&mut state, identity)?;
        reserve_call(record, maximum_call_debit, observed_root_cycles)?;
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    /// Persist the sole controller normalization intent before the management call.
    pub fn issue_controllers(
        identity: PoolImportIdentity,
        observed: &PoolImportObservationView,
        budget: PoolImportCallBudgetView,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let record = required_mut(&mut state, identity)?;
        let index = source_index(record, observed.canister_id)?;
        if record.phase != PoolImportPhaseRecord::Reserved
            || record.progress[index] != PoolImportResetProgressRecord::AwaitingHandoff
            || CanisterPoolStore::get(&observed.canister_id).is_some()
        {
            return Err(InternalError::conflict());
        }
        require_observation(record, index, observed, 1, false, true)?;
        reserve_call(
            record,
            budget.maximum_debit_cycles,
            budget.observed_root_cycles,
        )?;
        record.progress[index] = PoolImportResetProgressRecord::ControllersIssued {
            before_total_cycles: source_total(observed.cycles, observed.reserved_cycles)?,
            sender_canister_version: budget.sender_canister_version,
        };
        let asset = CanisterPoolAssetRecord {
            creation_receipt: None,
            cycles: Cycles::new(observed.cycles),
            origin: CanisterPoolAssetOriginRecord::Imported,
            status: CanisterPoolAssetStatusRecord::PendingReset,
            last_recycle: None,
            added_at_ns: now_ns,
            updated_at_ns: now_ns,
        };
        CanisterPoolStore::insert(observed.canister_id, asset);
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    /// Confirm an issued normalization from exact evidence, including after a lost reply.
    pub fn observe_controllers(
        identity: PoolImportIdentity,
        observed: &PoolImportObservationView,
        history: &PoolImportHistoryView,
    ) -> Result<(), InternalError> {
        advance(identity, observed, |record, index| {
            let PoolImportResetProgressRecord::ControllersIssued {
                before_total_cycles,
                sender_canister_version,
            } = record.progress[index]
            else {
                return Err(InternalError::conflict());
            };
            require_history(
                record,
                observed,
                history,
                sender_canister_version,
                PoolImportHistoryKind::Controllers,
            )?;
            require_observation(record, index, observed, 2, false, false)?;
            if source_total(observed.cycles, observed.reserved_cycles)? > before_total_cycles {
                return Err(InternalError::conflict());
            }
            record.progress[index] = PoolImportResetProgressRecord::ControllersConfirmed {
                retained_total_cycles: source_total(observed.cycles, observed.reserved_cycles)?,
            };
            Ok(())
        })
    }

    /// Persist the sole destructive reset intent, retaining its original source balance.
    pub fn issue_uninstall(
        identity: PoolImportIdentity,
        observed: &PoolImportObservationView,
        budget: PoolImportCallBudgetView,
    ) -> Result<(), InternalError> {
        advance(identity, observed, |record, index| {
            let PoolImportResetProgressRecord::ControllersConfirmed {
                retained_total_cycles,
            } = record.progress[index]
            else {
                return Err(InternalError::conflict());
            };
            require_observation(record, index, observed, 2, false, false)?;
            if source_total(observed.cycles, observed.reserved_cycles)? > retained_total_cycles {
                return Err(InternalError::conflict());
            }
            reserve_call(
                record,
                budget.maximum_debit_cycles,
                budget.observed_root_cycles,
            )?;
            record.progress[index] = PoolImportResetProgressRecord::UninstallIssued {
                before_total_cycles: source_total(observed.cycles, observed.reserved_cycles)?,
                sender_canister_version: budget.sender_canister_version,
            };
            Ok(())
        })
    }

    /// Retain cleared-code proof and Ready eligibility while the global allocation fence holds.
    pub fn observe_cleared(
        identity: PoolImportIdentity,
        observed: &PoolImportObservationView,
        history: &PoolImportHistoryView,
        now_ns: u64,
    ) -> Result<PoolImportSourceReceipt, InternalError> {
        let mut state = CanisterPoolStore::state();
        let record = required_mut(&mut state, identity)?;
        let index = source_index(record, observed.canister_id)?;
        if let PoolImportResetProgressRecord::Ready(receipt) = &record.progress[index] {
            return Ok(receipt.clone());
        }
        let PoolImportResetProgressRecord::UninstallIssued {
            before_total_cycles,
            sender_canister_version,
        } = record.progress[index]
        else {
            return Err(InternalError::conflict());
        };
        require_history(
            record,
            observed,
            history,
            sender_canister_version,
            PoolImportHistoryKind::Uninstall,
        )?;
        require_observation(record, index, observed, 3, true, false)?;
        if source_total(observed.cycles, observed.reserved_cycles)? > before_total_cycles {
            return Err(InternalError::conflict());
        }
        let mut asset = required_reserved_asset(observed.canister_id)?;
        let receipt = PoolImportSourceReceipt {
            root_sender_canister_version: sender_canister_version,
            canister_id: observed.canister_id,
            canister_version: observed.canister_version,
            retained_cycles: observed.cycles,
            retained_reserved_cycles: observed.reserved_cycles,
            observed_debit_cycles: source_total(
                record.reservation.sources[index].observed_cycles,
                record.reservation.sources[index].observed_reserved_cycles,
            )? - source_total(observed.cycles, observed.reserved_cycles)?,
        };
        record.progress[index] = PoolImportResetProgressRecord::Ready(receipt.clone());
        if record
            .progress
            .iter()
            .all(|progress| matches!(progress, PoolImportResetProgressRecord::Ready(_)))
        {
            record.phase = PoolImportPhaseRecord::Ready;
        }
        asset.status = CanisterPoolAssetStatusRecord::Ready;
        asset.cycles = Cycles::new(observed.cycles);
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(observed.canister_id, asset);
        CanisterPoolStore::set_state(state);
        Ok(receipt)
    }

    /// Freeze terminal Root accounting after every source is cleared and all paid reads finish.
    pub fn settle(
        identity: PoolImportIdentity,
        native_cycles: u128,
        reserved_cycles: u128,
    ) -> Result<PoolImportStatus, InternalError> {
        let mut state = CanisterPoolStore::state();
        let record = required_mut(&mut state, identity)?;
        if record.root_receipt.is_some() {
            return Ok(status(record));
        }
        if record.phase != PoolImportPhaseRecord::Ready {
            return Err(InternalError::conflict());
        }
        let authority = &record.reservation;
        let debit = source_total(
            authority.observed_root_cycles,
            authority.observed_root_reserved_cycles,
        )?
        .checked_sub(source_total(native_cycles, reserved_cycles)?)
        .ok_or_else(InternalError::conflict)?;
        if debit > authority.maximum_root_debit_cycles
            || native_cycles < authority.minimum_root_cycles
        {
            return Err(InternalError::resource_exhausted());
        }
        record.last_root_cycles = native_cycles;
        record.root_receipt = Some(PoolImportRootReceipt {
            retained_cycles: native_cycles,
            retained_reserved_cycles: reserved_cycles,
            observed_debit_cycles: debit,
        });
        let response = status(record);
        CanisterPoolStore::set_state(state);
        Ok(response)
    }

    /// Release allocation only after the host retains exact inventory publication evidence.
    /// Exact replay returns the retained receipt even after the source becomes a workload.
    pub fn release(
        identity: PoolImportIdentity,
        publication_sha256: [u8; 32],
    ) -> Result<PoolImportStatus, InternalError> {
        if publication_sha256 == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        let mut state = CanisterPoolStore::state();
        let record = required_mut(&mut state, identity)?;
        match record.phase {
            PoolImportPhaseRecord::Ready if record.root_receipt.is_some() => {}
            PoolImportPhaseRecord::Released {
                publication_sha256: retained,
            } if retained == publication_sha256 => {
                return Ok(status(record));
            }
            _ => return Err(InternalError::conflict()),
        }
        for source in &record.reservation.sources {
            let asset =
                CanisterPoolStore::get(&source.canister_id).ok_or_else(InternalError::invariant)?;
            if asset.status != CanisterPoolAssetStatusRecord::Ready
                || asset.origin != CanisterPoolAssetOriginRecord::Imported
            {
                return Err(InternalError::conflict());
            }
        }
        record.phase = PoolImportPhaseRecord::Released { publication_sha256 };
        let response = status(record);
        state.bootstrap_import = None;
        CanisterPoolStore::set_state(state);
        Ok(response)
    }
}

fn validate_reservation(
    request: &PoolImportReservation,
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<(), InternalError> {
    source_total(
        request.observed_root_cycles,
        request.observed_root_reserved_cycles,
    )?;
    let valid_principal =
        |id: Principal| id != Principal::anonymous() && id != Principal::management_canister();
    if request.plan_sha256 == [0; 32]
        || request.root_authority_sha256 == [0; 32]
        || ![request.root, request.operator, request.subnet]
            .into_iter()
            .all(valid_principal)
        || request.root == request.operator
        || request.sources.is_empty()
        || request.sources.len() > MAX_FLEET_CAPACITY_IMPORT_SOURCES
        || request.maximum_paid_calls == 0
        || request.maximum_root_debit_cycles == 0
        || request.minimum_root_cycles == 0
    {
        return Err(InternalError::invalid_input());
    }
    let controllers = &request.final_controllers;
    let canonical = controllers.windows(2).all(|pair| pair[0] < pair[1]);
    if controllers.is_empty()
        || controllers.len() > 10
        || !canonical
        || !controllers.iter().copied().all(valid_principal)
        || !controllers.contains(&request.root)
    {
        return Err(InternalError::invalid_input());
    }
    let mut transitional = controllers.clone();
    transitional.push(request.operator);
    transitional.sort_unstable();
    transitional.dedup();
    if transitional.len() > 10 || transitional != request.transitional_controllers {
        return Err(InternalError::invalid_input());
    }
    if request
        .observed_root_cycles
        .checked_sub(request.maximum_root_debit_cycles)
        .is_none_or(|remaining| remaining < request.minimum_root_cycles)
    {
        return Err(InternalError::resource_exhausted());
    }
    if !request
        .sources
        .windows(2)
        .all(|pair| pair[0].canister_id < pair[1].canister_id)
    {
        return Err(InternalError::invalid_input());
    }
    for source in &request.sources {
        source_total(source.observed_cycles, source.observed_reserved_cycles)?;
        if !valid_principal(source.canister_id)
            || source.canister_id == request.root
            || source.canister_id == request.operator
            || request.final_controllers.contains(&source.canister_id)
            || source.disposition_sha256 == [0; 32]
            || source.maximum_debit_cycles == 0
            || source.canister_version.checked_add(3).is_none()
            || (source.module_sha256.is_some() && !source.stopped)
            || source.minimum_ready_cycles < config.canister_cycles.to_u128()
        {
            return Err(InternalError::invalid_input());
        }
        if source
            .observed_cycles
            .checked_sub(source.maximum_debit_cycles)
            .is_none_or(|remaining| remaining < source.minimum_ready_cycles)
        {
            return Err(InternalError::resource_exhausted());
        }
    }
    Ok(())
}

fn require_quiet_pool(state: &CanisterPoolStateRecord) -> Result<(), InternalError> {
    if state.creation.is_some() || state.handoff.is_some() {
        return Err(InternalError::conflict());
    }
    for entry in CanisterPoolStore::export().entries {
        if !matches!(
            entry.asset.status,
            CanisterPoolAssetStatusRecord::Store
                | CanisterPoolAssetStatusRecord::Ready
                | CanisterPoolAssetStatusRecord::Workload(_)
                | CanisterPoolAssetStatusRecord::Failed(_)
        ) {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn required(
    state: &CanisterPoolStateRecord,
    identity: PoolImportIdentity,
) -> Result<&PoolImportRecord, InternalError> {
    state
        .capacity_import
        .as_ref()
        .filter(|record| identity_matches(record, identity))
        .ok_or_else(InternalError::conflict)
}

fn required_mut(
    state: &mut CanisterPoolStateRecord,
    identity: PoolImportIdentity,
) -> Result<&mut PoolImportRecord, InternalError> {
    state
        .capacity_import
        .as_mut()
        .filter(|record| identity_matches(record, identity))
        .ok_or_else(InternalError::conflict)
}

fn identity_matches(record: &PoolImportRecord, identity: PoolImportIdentity) -> bool {
    PoolImportIdentity {
        sequence: record.reservation.sequence,
        plan_sha256: record.reservation.plan_sha256,
    } == identity
}

fn source_index(record: &PoolImportRecord, id: Principal) -> Result<usize, InternalError> {
    record
        .reservation
        .sources
        .iter()
        .position(|source| source.canister_id == id)
        .ok_or_else(InternalError::conflict)
}

fn advance(
    identity: PoolImportIdentity,
    observed: &PoolImportObservationView,
    transition: impl FnOnce(&mut PoolImportRecord, usize) -> Result<(), InternalError>,
) -> Result<(), InternalError> {
    let mut state = CanisterPoolStore::state();
    let record = required_mut(&mut state, identity)?;
    if record.phase != PoolImportPhaseRecord::Reserved {
        return Err(InternalError::conflict());
    }
    let index = source_index(record, observed.canister_id)?;
    required_reserved_asset(observed.canister_id)?;
    transition(record, index)?;
    CanisterPoolStore::set_state(state);
    Ok(())
}

fn required_reserved_asset(
    id: Principal,
) -> Result<crate::storage::stable::canister_pool::CanisterPoolAssetRecord, InternalError> {
    CanisterPoolStore::get(&id)
        .filter(|asset| {
            asset.origin == CanisterPoolAssetOriginRecord::Imported
                && asset.status == CanisterPoolAssetStatusRecord::PendingReset
        })
        .ok_or_else(InternalError::conflict)
}

fn require_observation(
    record: &PoolImportRecord,
    index: usize,
    observed: &PoolImportObservationView,
    version_delta: u64,
    cleared: bool,
    transitional: bool,
) -> Result<(), InternalError> {
    let source: &PoolImportSource = &record.reservation.sources[index];
    let expected_controllers = if transitional {
        &record.reservation.transitional_controllers
    } else {
        &record.reservation.final_controllers
    };
    let expected_module = if cleared { None } else { source.module_sha256 };
    let expected_version = source
        .canister_version
        .checked_add(version_delta)
        .ok_or_else(InternalError::invariant)?;
    let expected = (
        source.canister_id,
        expected_version,
        expected_module,
        expected_controllers,
        source.stopped,
    );
    let actual = (
        observed.canister_id,
        observed.canister_version,
        observed.module_sha256,
        &observed.controllers,
        observed.stopped,
    );
    if actual != expected || observed.snapshots_size_bytes != 0 {
        return Err(InternalError::conflict());
    }
    let debit = source_total(source.observed_cycles, source.observed_reserved_cycles)?
        .checked_sub(source_total(observed.cycles, observed.reserved_cycles)?)
        .ok_or_else(InternalError::conflict)?;
    if debit > source.maximum_debit_cycles || observed.cycles < source.minimum_ready_cycles {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}

fn require_completion_fits(state: &CanisterPoolStateRecord) -> Result<(), InternalError> {
    let mut expanded = state.clone();
    let record = expanded
        .capacity_import
        .as_mut()
        .ok_or_else(InternalError::invariant)?;
    record.paid_calls = u32::MAX;
    record.reserved_debit_cycles = u128::MAX;
    record.last_root_cycles = u128::MAX;
    record.root_receipt = Some(PoolImportRootReceipt {
        retained_cycles: u128::MAX,
        retained_reserved_cycles: u128::MAX,
        observed_debit_cycles: u128::MAX,
    });
    record.phase = PoolImportPhaseRecord::Released {
        publication_sha256: [u8::MAX; 32],
    };
    record.progress = record
        .reservation
        .sources
        .iter()
        .map(|source| {
            PoolImportResetProgressRecord::Ready(PoolImportSourceReceipt {
                canister_id: source.canister_id,
                canister_version: u64::MAX,
                root_sender_canister_version: u64::MAX,
                retained_cycles: u128::MAX,
                retained_reserved_cycles: u128::MAX,
                observed_debit_cycles: u128::MAX,
            })
        })
        .collect();
    // Keep headroom for the existing refill/handoff fields after release.
    if expanded.to_bytes().len() + 4_096 > CANISTER_POOL_STATE_MAX_BYTES as usize {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}

fn status(record: &PoolImportRecord) -> PoolImportStatus {
    PoolImportStatus {
        reserved_at_ns: record.reserved_at_ns,
        root_receipt: record.root_receipt.clone(),
        reservation: record.reservation.clone(),
        progress: record
            .progress
            .iter()
            .map(|progress| match progress {
                PoolImportResetProgressRecord::AwaitingHandoff => {
                    PoolImportSourceProgress::AwaitingHandoff
                }
                PoolImportResetProgressRecord::ControllersIssued { .. } => {
                    PoolImportSourceProgress::ControllersIssued
                }
                PoolImportResetProgressRecord::ControllersConfirmed { .. } => {
                    PoolImportSourceProgress::ControllersConfirmed
                }
                PoolImportResetProgressRecord::UninstallIssued { .. } => {
                    PoolImportSourceProgress::UninstallIssued
                }
                PoolImportResetProgressRecord::Ready(receipt) => {
                    PoolImportSourceProgress::Ready(receipt.clone())
                }
            })
            .collect(),
        phase: match record.phase {
            PoolImportPhaseRecord::Reserved => PoolImportPhase::Reserved,
            PoolImportPhaseRecord::Ready => PoolImportPhase::Ready,
            PoolImportPhaseRecord::Released { publication_sha256 } => {
                PoolImportPhase::Released { publication_sha256 }
            }
        },
        paid_calls: record.paid_calls,
        reserved_debit_cycles: record.reserved_debit_cycles,
        last_root_cycles: record.last_root_cycles,
    }
}

fn reserve_call(
    record: &mut PoolImportRecord,
    maximum_call_debit: u128,
    observed_root_cycles: u128,
) -> Result<(), InternalError> {
    if record.root_receipt.is_some()
        || matches!(record.phase, PoolImportPhaseRecord::Released { .. })
    {
        return Err(InternalError::conflict());
    }
    let authority = &record.reservation;
    let debit = authority
        .observed_root_cycles
        .checked_sub(observed_root_cycles)
        .ok_or_else(InternalError::conflict)?;
    if observed_root_cycles < authority.minimum_root_cycles
        || debit > authority.maximum_root_debit_cycles
    {
        return Err(InternalError::conflict());
    }
    let paid_calls = record
        .paid_calls
        .checked_add(1)
        .filter(|count| *count <= authority.maximum_paid_calls)
        .ok_or_else(InternalError::resource_exhausted)?;
    let reserved = record
        .reserved_debit_cycles
        .max(debit)
        .checked_add(maximum_call_debit)
        .filter(|amount| *amount <= authority.maximum_root_debit_cycles)
        .ok_or_else(InternalError::resource_exhausted)?;
    if maximum_call_debit == 0
        || observed_root_cycles.saturating_sub(maximum_call_debit) < authority.minimum_root_cycles
    {
        return Err(InternalError::resource_exhausted());
    }
    record.paid_calls = paid_calls;
    record.reserved_debit_cycles = reserved;
    record.last_root_cycles = observed_root_cycles;
    Ok(())
}

fn require_history(
    record: &PoolImportRecord,
    observed: &PoolImportObservationView,
    history: &PoolImportHistoryView,
    sender_canister_version: u64,
    kind: PoolImportHistoryKind,
) -> Result<(), InternalError> {
    let expected = PoolImportHistoryView {
        canister_id: observed.canister_id,
        canister_version: observed.canister_version,
        module_sha256: observed.module_sha256,
        controllers: observed.controllers.clone(),
        originator: record.reservation.root,
        sender_canister_version,
        kind,
    };
    if *history != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn source_total(cycles: u128, reserved_cycles: u128) -> Result<u128, InternalError> {
    cycles
        .checked_add(reserved_cycles)
        .ok_or_else(InternalError::resource_exhausted)
}
