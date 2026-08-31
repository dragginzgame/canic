//! Deterministic state access and DTO conversion for root-owned physical Canisters.

use crate::storage::stable::canister_pool::{
    CanisterPoolAssetOriginRecord, CanisterPoolAssetRecord, CanisterPoolAssetStatusRecord,
    CanisterPoolClaimRecord, CanisterPoolCreationFailureRecord, CanisterPoolCreationProgressRecord,
    CanisterPoolCreationRecord, CanisterPoolHandoffReceiptRecord, CanisterPoolHandoffRecord,
    CanisterPoolLedgerRecoveryArtifactRecord, CanisterPoolLedgerRecoveryAuthorityRecord,
    CanisterPoolLedgerRecoveryPhaseRecord, CanisterPoolLedgerRecoveryReceiptRecord,
    CanisterPoolLedgerRecoveryRecord, CanisterPoolRecycleResetRecord, CanisterPoolStore,
};
use crate::view::canister_pool::{
    CanisterPoolCreationFailureView, CanisterPoolCreationProgressView, CanisterPoolCreationView,
    CanisterPoolHandoffView,
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::error::InternalError,
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    dto::pool::{
        CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus, CanisterPoolClaim,
        CanisterPoolCreation, CanisterPoolCreationFailure, CanisterPoolCreationProgress,
        CanisterPoolHandoff, CanisterPoolRecycleReset, CanisterPoolResponse,
        PoolLedgerRecoveryArtifact, PoolLedgerRecoveryPhase, PoolLedgerRecoveryReceipt,
        PoolLedgerRecoveryRequest, PoolLedgerRecoveryStatusResponse,
    },
    ids::{ComponentInstanceId, FleetSubnetCanisterPoolConfig},
};
use std::collections::BTreeSet;

/// Stable identity of one Component allocation claiming a prepaid asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterPoolClaimKey {
    pub component: ComponentInstanceId,
    pub operation_id: [u8; 32],
}

/// Complete protected identity of one Cycles Ledger pool-refill request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterPoolCreationAuthority {
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub placement_subnet: Principal,
    pub root: Principal,
    pub ledger_amount: Cycles,
    pub created_at_time_ns: u64,
}

/// Exact workflow action after durably fencing one pool asset for reset or reinspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterPoolResetPreparation {
    Ready,
    Reinspect,
    Reset,
}

/// Exact durable boundary selected by the pool Ledger recovery workflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterPoolLedgerRecoveryTransition {
    HelperInstallIssued,
    HelperInstalled,
    WithdrawalIssued,
    WithdrawalVerified { block_index: u64 },
    HelperUninstallIssued { block_index: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadyReinspectionPolicy {
    OnlyWhenUnderfunded,
    AlwaysForImported,
}

/// Mechanical state facade for the Fleet Subnet Root's exclusive physical inventory.
pub struct CanisterPoolOps;

impl CanisterPoolOps {
    pub fn initialize_store(canister_id: Principal, now_ns: u64) -> Result<(), InternalError> {
        match CanisterPoolStore::get(&canister_id) {
            Some(existing)
                if existing.origin == CanisterPoolAssetOriginRecord::InfrastructureStore
                    && matches!(
                        existing.status,
                        CanisterPoolAssetStatusRecord::Store
                            | CanisterPoolAssetStatusRecord::StoreDeletionPending { .. }
                    ) =>
            {
                Ok(())
            }
            Some(_) => Err(InternalError::conflict()),
            None => {
                CanisterPoolStore::insert(
                    canister_id,
                    CanisterPoolAssetRecord {
                        cycles: Cycles::default(),
                        origin: CanisterPoolAssetOriginRecord::InfrastructureStore,
                        status: CanisterPoolAssetStatusRecord::Store,
                        last_recycle: None,
                        added_at_ns: now_ns,
                        updated_at_ns: now_ns,
                    },
                );
                Ok(())
            }
        }
    }

    pub fn initialize_imports(
        config: &FleetSubnetCanisterPoolConfig,
        imports: &[Principal],
        now_ns: u64,
    ) -> Result<(), InternalError> {
        validate_config(config)?;
        if imports.len() > config.maximum_size as usize {
            return Err(InternalError::invalid_input());
        }
        let unique = imports.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != imports.len() {
            return Err(InternalError::invalid_input());
        }

        for canister_id in imports {
            match CanisterPoolStore::get(canister_id) {
                Some(existing) if existing.origin == CanisterPoolAssetOriginRecord::Imported => {}
                Some(_) => {
                    return Err(InternalError::conflict());
                }
                None => {
                    validate_new_asset_capacity(config, *canister_id)?;
                    CanisterPoolStore::insert(
                        *canister_id,
                        CanisterPoolAssetRecord {
                            cycles: Cycles::default(),
                            origin: CanisterPoolAssetOriginRecord::Imported,
                            status: CanisterPoolAssetStatusRecord::PendingReset,
                            last_recycle: None,
                            added_at_ns: now_ns,
                            updated_at_ns: now_ns,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    pub fn register_recycled_pending(
        canister_id: Principal,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        match &asset.status {
            CanisterPoolAssetStatusRecord::Workload(claim) => {
                asset.origin = CanisterPoolAssetOriginRecord::Recycled;
                asset.status = CanisterPoolAssetStatusRecord::Recycling {
                    claim: claim.clone(),
                    reset: CanisterPoolRecycleResetRecord::Pending,
                };
                asset.updated_at_ns = now_ns;
                CanisterPoolStore::insert(canister_id, asset);
                Ok(())
            }
            CanisterPoolAssetStatusRecord::Recycling { .. }
                if asset.origin == CanisterPoolAssetOriginRecord::Recycled =>
            {
                Ok(())
            }
            _ => Err(InternalError::conflict()),
        }
    }

    pub fn mark_ready(
        canister_id: Principal,
        cycles: Cycles,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        asset.cycles = cycles;
        asset.status = match asset.status {
            CanisterPoolAssetStatusRecord::PendingReset
            | CanisterPoolAssetStatusRecord::Failed(_) => CanisterPoolAssetStatusRecord::Ready,
            CanisterPoolAssetStatusRecord::Recycling { claim, .. } => {
                CanisterPoolAssetStatusRecord::Recycling {
                    claim,
                    reset: CanisterPoolRecycleResetRecord::Ready,
                }
            }
            _ => {
                return Err(InternalError::conflict());
            }
        };
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
    }

    pub fn mark_failed(
        canister_id: Principal,
        observed_cycles: Option<Cycles>,
        reason: String,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        if let Some(cycles) = observed_cycles {
            asset.cycles = cycles;
        }
        asset.status = match asset.status {
            CanisterPoolAssetStatusRecord::PendingReset
            | CanisterPoolAssetStatusRecord::Failed(_) => {
                CanisterPoolAssetStatusRecord::Failed(reason)
            }
            CanisterPoolAssetStatusRecord::Recycling { claim, .. } => {
                CanisterPoolAssetStatusRecord::Recycling {
                    claim,
                    reset: CanisterPoolRecycleResetRecord::Failed(reason),
                }
            }
            _ => {
                return Err(InternalError::conflict());
            }
        };
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
    }

    pub fn retry_reset(
        canister_id: Principal,
        required_cycles: &Cycles,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        asset.status = match asset.status {
            CanisterPoolAssetStatusRecord::Ready if asset.cycles < *required_cycles => {
                CanisterPoolAssetStatusRecord::PendingReset
            }
            CanisterPoolAssetStatusRecord::PendingReset
            | CanisterPoolAssetStatusRecord::Recycling {
                reset: CanisterPoolRecycleResetRecord::Pending,
                ..
            } => return Ok(()),
            CanisterPoolAssetStatusRecord::Failed(_) => CanisterPoolAssetStatusRecord::PendingReset,
            CanisterPoolAssetStatusRecord::Recycling {
                claim,
                reset: CanisterPoolRecycleResetRecord::Failed(_),
            } => CanisterPoolAssetStatusRecord::Recycling {
                claim,
                reset: CanisterPoolRecycleResetRecord::Pending,
            },
            _ => {
                return Err(InternalError::conflict());
            }
        };
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
    }

    /// Fence an undersized Ready row before the workflow re-inspects its live balance.
    pub fn prepare_ready_reinspection(
        canister_id: Principal,
        required_cycles: &Cycles,
        now_ns: u64,
    ) -> Result<CanisterPoolResetPreparation, InternalError> {
        Self::prepare_reinspection(
            canister_id,
            required_cycles,
            now_ns,
            ReadyReinspectionPolicy::OnlyWhenUnderfunded,
        )
    }

    /// Fence an explicitly re-imported asset before refreshing its retained live balance.
    pub fn prepare_import_reinspection(
        canister_id: Principal,
        required_cycles: &Cycles,
        now_ns: u64,
    ) -> Result<CanisterPoolResetPreparation, InternalError> {
        Self::prepare_reinspection(
            canister_id,
            required_cycles,
            now_ns,
            ReadyReinspectionPolicy::AlwaysForImported,
        )
    }

    fn prepare_reinspection(
        canister_id: Principal,
        required_cycles: &Cycles,
        now_ns: u64,
        policy: ReadyReinspectionPolicy,
    ) -> Result<CanisterPoolResetPreparation, InternalError> {
        let mut asset = required_asset(canister_id)?;
        match asset.status {
            CanisterPoolAssetStatusRecord::Ready
                if policy == ReadyReinspectionPolicy::AlwaysForImported
                    && asset.origin == CanisterPoolAssetOriginRecord::Imported =>
            {
                asset.status = CanisterPoolAssetStatusRecord::PendingReset;
                asset.updated_at_ns = now_ns;
                CanisterPoolStore::insert(canister_id, asset);
                Ok(CanisterPoolResetPreparation::Reinspect)
            }
            CanisterPoolAssetStatusRecord::Ready if asset.cycles >= *required_cycles => {
                Ok(CanisterPoolResetPreparation::Ready)
            }
            CanisterPoolAssetStatusRecord::Ready => {
                asset.status = CanisterPoolAssetStatusRecord::PendingReset;
                asset.updated_at_ns = now_ns;
                CanisterPoolStore::insert(canister_id, asset);
                Ok(CanisterPoolResetPreparation::Reinspect)
            }
            CanisterPoolAssetStatusRecord::PendingReset
            | CanisterPoolAssetStatusRecord::Failed(_)
                if asset.cycles > Cycles::default() =>
            {
                Ok(CanisterPoolResetPreparation::Reinspect)
            }
            CanisterPoolAssetStatusRecord::PendingReset
            | CanisterPoolAssetStatusRecord::Failed(_)
            | CanisterPoolAssetStatusRecord::Recycling { .. } => {
                Ok(CanisterPoolResetPreparation::Reset)
            }
            _ => Err(InternalError::conflict()),
        }
    }

    #[must_use]
    pub fn pending_reset_canisters() -> Vec<Principal> {
        CanisterPoolStore::export()
            .entries
            .into_iter()
            .filter_map(|entry| {
                matches!(
                    entry.asset.status,
                    CanisterPoolAssetStatusRecord::PendingReset
                        | CanisterPoolAssetStatusRecord::Recycling {
                            reset: CanisterPoolRecycleResetRecord::Pending,
                            ..
                        }
                )
                .then_some(entry.canister_id)
            })
            .collect()
    }

    pub fn claim_smallest_sufficient_ready(
        claim: &CanisterPoolClaimKey,
        required_cycles: &Cycles,
        now_ns: u64,
    ) -> Result<Option<Principal>, InternalError> {
        let data = CanisterPoolStore::export();
        let expected_claim = claim_record(claim);
        let existing = data
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    &entry.asset.status,
                    CanisterPoolAssetStatusRecord::Claimed(current) if current == &expected_claim
                )
            })
            .map(|entry| entry.canister_id)
            .collect::<Vec<_>>();
        if existing.len() > 1 {
            return Err(InternalError::invariant());
        }
        if let Some(canister_id) = existing.first() {
            return Ok(Some(*canister_id));
        }

        let selected = data
            .entries
            .into_iter()
            .filter(|entry| {
                matches!(entry.asset.status, CanisterPoolAssetStatusRecord::Ready)
                    && entry.asset.cycles >= *required_cycles
            })
            .min_by(|left, right| {
                left.asset
                    .cycles
                    .cmp(&right.asset.cycles)
                    .then_with(|| left.asset.added_at_ns.cmp(&right.asset.added_at_ns))
                    .then_with(|| {
                        left.canister_id
                            .as_slice()
                            .cmp(right.canister_id.as_slice())
                    })
            });
        let Some(mut selected) = selected else {
            return Ok(None);
        };
        selected.asset.status = CanisterPoolAssetStatusRecord::Claimed(expected_claim);
        selected.asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(selected.canister_id, selected.asset);
        Ok(Some(selected.canister_id))
    }

    pub fn claimed_canister(
        claim: &CanisterPoolClaimKey,
    ) -> Result<Option<Principal>, InternalError> {
        let expected = claim_record(claim);
        let claimed = CanisterPoolStore::export()
            .entries
            .into_iter()
            .filter_map(|entry| {
                matches!(
                    entry.asset.status,
                    CanisterPoolAssetStatusRecord::Claimed(ref current) if current == &expected
                )
                .then_some(entry.canister_id)
            })
            .collect::<Vec<_>>();
        match claimed.as_slice() {
            [] => Ok(None),
            [canister_id] => Ok(Some(*canister_id)),
            _ => Err(InternalError::invariant()),
        }
    }

    pub fn finalize_claim(
        claim: &CanisterPoolClaimKey,
        canister_id: Principal,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let expected = claim_record(claim);
        let mut asset = required_asset(canister_id)?;
        match &asset.status {
            CanisterPoolAssetStatusRecord::Claimed(current) if current == &expected => {
                asset.status = CanisterPoolAssetStatusRecord::Workload(expected);
                asset.updated_at_ns = now_ns;
                CanisterPoolStore::insert(canister_id, asset);
                Ok(())
            }
            CanisterPoolAssetStatusRecord::Workload(current) if current == &expected => Ok(()),
            _ => Err(InternalError::conflict()),
        }
    }

    /// Fence one empty pool asset and retain exact authority before any recovery effect.
    pub fn prepare_ledger_recovery(
        request: &PoolLedgerRecoveryRequest,
        initial_native_cycles: Cycles,
        prepared_at_ns: u64,
    ) -> Result<PoolLedgerRecoveryStatusResponse, InternalError> {
        validate_ledger_recovery_request(request)?;
        let authority = ledger_recovery_authority_from_dto(request);
        let mut state = CanisterPoolStore::state();
        if let Some(receipt) = &state.last_ledger_recovery {
            if receipt.authority == authority {
                return Ok(ledger_recovery_receipt_status(receipt));
            }
            if receipt.authority.operation_id == request.operation_id {
                return Err(InternalError::conflict());
            }
        }
        if let Some(current) = &state.ledger_recovery {
            if current.authority == authority {
                return Ok(ledger_recovery_status(current));
            }
            return Err(InternalError::conflict());
        }
        if state.creation.is_some() || state.handoff.is_some() {
            return Err(InternalError::conflict());
        }
        let mut asset = required_asset(request.canister_id)?;
        match asset.status {
            CanisterPoolAssetStatusRecord::PendingReset
            | CanisterPoolAssetStatusRecord::Ready
            | CanisterPoolAssetStatusRecord::Failed(_) => {}
            CanisterPoolAssetStatusRecord::RecoveringLedger { operation_id }
                if operation_id == request.operation_id => {}
            _ => return Err(InternalError::conflict()),
        }
        asset.cycles = initial_native_cycles.clone();
        asset.status = CanisterPoolAssetStatusRecord::RecoveringLedger {
            operation_id: request.operation_id,
        };
        asset.updated_at_ns = prepared_at_ns;
        CanisterPoolStore::insert(request.canister_id, asset);
        state.ledger_recovery = Some(CanisterPoolLedgerRecoveryRecord {
            authority,
            initial_native_cycles,
            phase: CanisterPoolLedgerRecoveryPhaseRecord::Prepared,
            prepared_at_ns,
        });
        CanisterPoolStore::set_state(state);
        Ok(ledger_recovery_status(
            &CanisterPoolStore::state()
                .ledger_recovery
                .expect("recovery was retained above"),
        ))
    }

    /// Return current or terminal recovery status for one exact operation identity.
    pub fn ledger_recovery_status_by_operation(
        operation_id: [u8; 32],
    ) -> Option<PoolLedgerRecoveryStatusResponse> {
        let state = CanisterPoolStore::state();
        state
            .ledger_recovery
            .as_ref()
            .filter(|current| current.authority.operation_id == operation_id)
            .map(ledger_recovery_status)
            .or_else(|| {
                state
                    .last_ledger_recovery
                    .as_ref()
                    .filter(|receipt| receipt.authority.operation_id == operation_id)
                    .map(ledger_recovery_receipt_status)
            })
    }

    /// Advance one exact durable phase without performing a platform effect.
    pub fn advance_ledger_recovery(
        request: &PoolLedgerRecoveryRequest,
        transition: CanisterPoolLedgerRecoveryTransition,
    ) -> Result<PoolLedgerRecoveryStatusResponse, InternalError> {
        let authority = ledger_recovery_authority_from_dto(request);
        let mut state = CanisterPoolStore::state();
        let current = state
            .ledger_recovery
            .as_mut()
            .ok_or_else(InternalError::unavailable)?;
        if current.authority != authority {
            return Err(InternalError::conflict());
        }
        let next = match (current.phase, transition) {
            (
                CanisterPoolLedgerRecoveryPhaseRecord::Prepared,
                CanisterPoolLedgerRecoveryTransition::HelperInstallIssued,
            ) => CanisterPoolLedgerRecoveryPhaseRecord::HelperInstallIssued,
            (
                CanisterPoolLedgerRecoveryPhaseRecord::HelperInstallIssued,
                CanisterPoolLedgerRecoveryTransition::HelperInstalled,
            ) => CanisterPoolLedgerRecoveryPhaseRecord::HelperInstalled,
            (
                CanisterPoolLedgerRecoveryPhaseRecord::HelperInstalled,
                CanisterPoolLedgerRecoveryTransition::WithdrawalIssued,
            ) => CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalIssued,
            (
                CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalIssued,
                CanisterPoolLedgerRecoveryTransition::WithdrawalVerified { block_index },
            ) => CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalVerified { block_index },
            (
                CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalVerified { block_index },
                CanisterPoolLedgerRecoveryTransition::HelperUninstallIssued {
                    block_index: requested,
                },
            ) if block_index == requested => {
                CanisterPoolLedgerRecoveryPhaseRecord::HelperUninstallIssued { block_index }
            }
            (existing, requested) if transition_matches(existing, requested) => existing,
            _ => return Err(InternalError::conflict()),
        };
        current.phase = next;
        let response = ledger_recovery_status(current);
        CanisterPoolStore::set_state(state);
        Ok(response)
    }

    /// Commit one recovery only after the helper is absent and both balance sides were proven.
    ///
    /// A distinct later recovery may rotate the bounded terminal slot; reusing an operation ID
    /// with different authority remains a conflict.
    pub fn complete_ledger_recovery(
        request: &PoolLedgerRecoveryRequest,
        final_native_cycles: Cycles,
        completed_at_ns: u64,
    ) -> Result<PoolLedgerRecoveryReceipt, InternalError> {
        let authority = ledger_recovery_authority_from_dto(request);
        let mut state = CanisterPoolStore::state();
        if let Some(existing) = &state.last_ledger_recovery {
            if existing.authority == authority {
                return Ok(ledger_recovery_receipt_to_dto(existing));
            }
            if existing.authority.operation_id == request.operation_id {
                return Err(InternalError::conflict());
            }
        }
        let current = state
            .ledger_recovery
            .take()
            .ok_or_else(InternalError::unavailable)?;
        if current.authority != authority {
            return Err(InternalError::conflict());
        }
        let CanisterPoolLedgerRecoveryPhaseRecord::HelperUninstallIssued { block_index } =
            current.phase
        else {
            return Err(InternalError::conflict());
        };
        let mut asset = required_asset(request.canister_id)?;
        if asset.status
            != (CanisterPoolAssetStatusRecord::RecoveringLedger {
                operation_id: request.operation_id,
            })
        {
            return Err(InternalError::conflict());
        }
        asset.cycles = final_native_cycles.clone();
        asset.status = CanisterPoolAssetStatusRecord::Ready;
        asset.updated_at_ns = completed_at_ns;
        CanisterPoolStore::insert(request.canister_id, asset);
        let receipt = CanisterPoolLedgerRecoveryReceiptRecord {
            authority,
            block_index,
            completed_at_ns,
            final_native_cycles,
            initial_native_cycles: current.initial_native_cycles,
        };
        let response = ledger_recovery_receipt_to_dto(&receipt);
        state.last_ledger_recovery = Some(receipt);
        CanisterPoolStore::set_state(state);
        Ok(response)
    }

    pub fn response(
        config: FleetSubnetCanisterPoolConfig,
        start_after: Option<Principal>,
        limit: usize,
    ) -> CanisterPoolResponse {
        let data = CanisterPoolStore::export();
        let state = data.state;
        let mut ready = 0_u32;
        let mut store = 0_u32;
        let mut store_deletion_pending = 0_u32;
        let mut pending_reset = 0_u32;
        let mut claimed = 0_u32;
        let mut workload = 0_u32;
        let mut recycling = 0_u32;
        let mut recovering_ledger = 0_u32;
        let mut handing_off = 0_u32;
        let mut failed = 0_u32;
        let all_entries: Vec<CanisterPoolAsset> = data
            .entries
            .into_iter()
            .map(|entry| {
                match &entry.asset.status {
                    CanisterPoolAssetStatusRecord::Store => store += 1,
                    CanisterPoolAssetStatusRecord::StoreDeletionPending { .. } => {
                        store_deletion_pending += 1;
                    }
                    CanisterPoolAssetStatusRecord::PendingReset => pending_reset += 1,
                    CanisterPoolAssetStatusRecord::Ready => ready += 1,
                    CanisterPoolAssetStatusRecord::Claimed(_) => claimed += 1,
                    CanisterPoolAssetStatusRecord::Workload(_) => workload += 1,
                    CanisterPoolAssetStatusRecord::Recycling { .. } => recycling += 1,
                    CanisterPoolAssetStatusRecord::RecoveringLedger { .. } => {
                        recovering_ledger += 1;
                    }
                    CanisterPoolAssetStatusRecord::HandingOff { .. } => handing_off += 1,
                    CanisterPoolAssetStatusRecord::Failed(_) => failed += 1,
                }
                asset_to_dto(entry.canister_id, entry.asset)
            })
            .collect();
        let tracked = count_as_u32(all_entries.len());
        let pooled = count_as_u32(
            all_entries
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.status,
                        CanisterPoolAssetStatus::PendingReset
                            | CanisterPoolAssetStatus::Ready
                            | CanisterPoolAssetStatus::RecoveringLedger { .. }
                            | CanisterPoolAssetStatus::HandingOff { .. }
                            | CanisterPoolAssetStatus::Failed { .. }
                    )
                })
                .count(),
        );
        let mut page = all_entries
            .into_iter()
            .filter(|entry| {
                start_after.is_none_or(|cursor| entry.canister_id.as_slice() > cursor.as_slice())
            })
            .take(limit.saturating_add(1))
            .collect::<Vec<_>>();
        let has_more = page.len() > limit;
        page.truncate(limit);
        let next_start_after = if has_more {
            page.last().map(|entry| entry.canister_id)
        } else {
            None
        };
        CanisterPoolResponse {
            surplus: pooled.saturating_sub(config.maximum_size),
            tracked,
            store,
            store_deletion_pending,
            pooled,
            workload,
            config,
            ready,
            pending_reset,
            claimed,
            recycling,
            recovering_ledger,
            handing_off,
            failed,
            completed_handoffs: CanisterPoolStore::handoff_receipt_count(),
            pending_creation: state.creation.map(creation_to_dto),
            pending_handoff: state.handoff.map(|handoff| CanisterPoolHandoff {
                canister_id: handoff.canister_id,
                recipient: handoff.recipient,
                prepared_at_ns: handoff.prepared_at_ns,
            }),
            entries: page,
            next_start_after,
        }
    }

    pub fn begin_creation(
        authority: CanisterPoolCreationAuthority,
        prepared_at_ns: u64,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        if state.ledger_recovery.is_some() || state.handoff.is_some() {
            return Err(InternalError::conflict());
        }
        if let Some(existing) = state.creation {
            if creation_authority(&existing) == authority {
                return Ok(());
            }
            return Err(InternalError::conflict());
        }
        if authority.created_at_time_ns <= state.last_creation_timestamp_ns {
            return Err(InternalError::conflict());
        }
        state.last_creation_timestamp_ns = authority.created_at_time_ns;
        state.creation = Some(CanisterPoolCreationRecord {
            operation_id: authority.operation_id,
            cycles_ledger: authority.cycles_ledger,
            placement_subnet: authority.placement_subnet,
            root: authority.root,
            ledger_amount: authority.ledger_amount,
            created_at_time_ns: authority.created_at_time_ns,
            prepared_at_ns,
            cost_guard_settlement: None,
            progress: CanisterPoolCreationProgressRecord::Intent {
                uncertain_result: false,
            },
        });
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn begin_creation_attempt(
        operation_id: [u8; 32],
        settlement: ReplayCostGuardSettlement,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_mut()
            .ok_or_else(InternalError::unavailable)?;
        if creation.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        if creation.cost_guard_settlement.is_some() {
            return Err(InternalError::conflict());
        }
        match creation.progress {
            CanisterPoolCreationProgressRecord::Intent { .. } => {
                creation.cost_guard_settlement = Some(settlement);
                creation.progress = CanisterPoolCreationProgressRecord::Intent {
                    uncertain_result: true,
                };
                CanisterPoolStore::set_state(state);
                Ok(())
            }
            _ => Err(InternalError::conflict()),
        }
    }

    pub fn finish_creation_attempt(
        operation_id: [u8; 32],
        settlement: ReplayCostGuardSettlement,
        uncertain_result: bool,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_mut()
            .ok_or_else(InternalError::unavailable)?;
        require_creation_attempt(creation, operation_id, settlement)?;
        creation.cost_guard_settlement = None;
        creation.progress = CanisterPoolCreationProgressRecord::Intent { uncertain_result };
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn mark_creation_created(
        operation_id: [u8; 32],
        block_index: u64,
        canister_id: Principal,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_mut()
            .ok_or_else(InternalError::unavailable)?;
        if creation.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        match creation.progress {
            CanisterPoolCreationProgressRecord::Intent { .. } => {
                creation.progress = CanisterPoolCreationProgressRecord::Created {
                    block_index,
                    canister_id,
                };
            }
            CanisterPoolCreationProgressRecord::Created {
                block_index: existing_block,
                canister_id: existing_canister,
            } if existing_block == block_index && existing_canister == canister_id => return Ok(()),
            _ => {
                return Err(InternalError::conflict());
            }
        }
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn settle_creation_attempt(
        operation_id: [u8; 32],
        settlement: ReplayCostGuardSettlement,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_mut()
            .ok_or_else(InternalError::unavailable)?;
        require_creation_operation(creation, operation_id)?;
        if creation.cost_guard_settlement != Some(settlement) {
            return Err(InternalError::conflict());
        }
        creation.cost_guard_settlement = None;
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn block_creation(
        operation_id: [u8; 32],
        failure: CanisterPoolCreationFailure,
    ) -> Result<(), InternalError> {
        let failure = creation_failure_from_dto(failure);
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_mut()
            .ok_or_else(InternalError::unavailable)?;
        require_creation_operation(creation, operation_id)?;
        if creation.cost_guard_settlement.is_some() {
            return Err(InternalError::conflict());
        }
        match creation.progress {
            CanisterPoolCreationProgressRecord::Intent { .. } => {
                creation.progress = CanisterPoolCreationProgressRecord::Blocked { failure };
            }
            CanisterPoolCreationProgressRecord::Blocked { failure: existing }
                if existing == failure =>
            {
                return Ok(());
            }
            _ => {
                return Err(InternalError::conflict());
            }
        }
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn register_created_pending_reset(
        operation_id: [u8; 32],
        canister_id: Principal,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let state = CanisterPoolStore::state();
        let creation = state.creation.ok_or_else(InternalError::unavailable)?;
        require_creation_operation(&creation, operation_id)?;
        let created_principal_is_exact = matches!(
            creation.progress,
            CanisterPoolCreationProgressRecord::Created {
                canister_id: created,
                ..
            } if created == canister_id
        );
        if !created_principal_is_exact {
            return Err(InternalError::conflict());
        }
        match CanisterPoolStore::get(&canister_id) {
            Some(existing) if created_asset_is_adopted(&existing) => Ok(()),
            Some(_) => Err(InternalError::conflict()),
            None => {
                CanisterPoolStore::insert(
                    canister_id,
                    CanisterPoolAssetRecord {
                        cycles: Cycles::default(),
                        origin: CanisterPoolAssetOriginRecord::Created,
                        status: CanisterPoolAssetStatusRecord::PendingReset,
                        last_recycle: None,
                        added_at_ns: now_ns,
                        updated_at_ns: now_ns,
                    },
                );
                Ok(())
            }
        }
    }

    pub fn commit_creation(operation_id: [u8; 32]) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        let CanisterPoolCreationProgressRecord::Created { canister_id, .. } = creation.progress
        else {
            return Err(InternalError::conflict());
        };
        require_creation_operation(creation, operation_id)?;
        require_creation_cost_settled(creation)?;
        require_created_inventory_adoption(canister_id)?;
        state.next_creation_sequence = state
            .next_creation_sequence
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        state.creation = None;
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn retry_blocked_creation() -> Result<[u8; 32], InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        let CanisterPoolCreationProgressRecord::Blocked { failure } = creation.progress else {
            return Err(InternalError::conflict());
        };
        if failure == CanisterPoolCreationFailureRecord::UnresolvedAfterLedgerWindow {
            return Err(InternalError::conflict());
        }
        let operation_id = creation.operation_id;
        state.next_creation_sequence = state
            .next_creation_sequence
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        state.creation = None;
        CanisterPoolStore::set_state(state);
        Ok(operation_id)
    }

    pub fn cancel_known_unapplied_creation() -> Result<[u8; 32], InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if !creation_is_known_unapplied(creation) {
            return Err(InternalError::conflict());
        }
        require_creation_cost_settled(creation)?;
        let operation_id = creation.operation_id;
        state.next_creation_sequence = state
            .next_creation_sequence
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        state.creation = None;
        CanisterPoolStore::set_state(state);
        Ok(operation_id)
    }

    pub fn rollover_known_expired_creation() -> Result<[u8; 32], InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if !creation_is_known_unapplied_intent(creation) {
            return Err(InternalError::conflict());
        }
        require_creation_cost_settled(creation)?;
        let operation_id = creation.operation_id;
        state.next_creation_sequence = state
            .next_creation_sequence
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        state.creation = None;
        CanisterPoolStore::set_state(state);
        Ok(operation_id)
    }

    #[must_use]
    pub fn next_creation_sequence() -> u64 {
        CanisterPoolStore::state().next_creation_sequence
    }

    pub fn next_creation_timestamp(now_ns: u64) -> Result<u64, InternalError> {
        CanisterPoolStore::state()
            .last_creation_timestamp_ns
            .checked_add(1)
            .map(|minimum| now_ns.max(minimum))
            .ok_or_else(InternalError::resource_exhausted)
    }

    #[must_use]
    pub fn has_pending_lifecycle_work() -> bool {
        let state = CanisterPoolStore::state();
        state.creation.is_some() || state.handoff.is_some() || state.ledger_recovery.is_some()
    }

    #[must_use]
    pub fn has_pending_ledger_recovery() -> bool {
        CanisterPoolStore::state().ledger_recovery.is_some()
    }

    #[must_use]
    pub fn pending_creation() -> Option<CanisterPoolCreationView> {
        CanisterPoolStore::state().creation.map(|creation| {
            let progress = match creation.progress {
                CanisterPoolCreationProgressRecord::Intent { uncertain_result } => {
                    CanisterPoolCreationProgressView::Intent { uncertain_result }
                }
                CanisterPoolCreationProgressRecord::Created {
                    block_index,
                    canister_id,
                } => CanisterPoolCreationProgressView::Created {
                    block_index,
                    canister_id,
                },
                CanisterPoolCreationProgressRecord::Blocked { failure } => {
                    CanisterPoolCreationProgressView::Blocked {
                        failure: creation_failure_to_view(failure),
                    }
                }
            };
            CanisterPoolCreationView {
                operation_id: creation.operation_id,
                cycles_ledger: creation.cycles_ledger,
                placement_subnet: creation.placement_subnet,
                root: creation.root,
                ledger_amount: creation.ledger_amount,
                created_at_time_ns: creation.created_at_time_ns,
                cost_guard_settlement: creation.cost_guard_settlement,
                progress,
            }
        })
    }

    pub fn begin_handoff(
        canister_id: Principal,
        recipient: Principal,
        prepared_at_ns: u64,
    ) -> Result<CanisterPoolHandoffView, InternalError> {
        if CanisterPoolStore::handoff_receipt(&canister_id).is_some() {
            return Err(InternalError::conflict());
        }
        let mut state = CanisterPoolStore::state();
        if state.creation.is_some() || state.ledger_recovery.is_some() {
            return Err(InternalError::unavailable());
        }
        if let Some(existing) = state.handoff {
            if existing.canister_id == canister_id && existing.recipient == recipient {
                let asset = required_asset(canister_id)?;
                if asset.status == (CanisterPoolAssetStatusRecord::HandingOff { recipient }) {
                    return Ok(CanisterPoolHandoffView {
                        canister_id,
                        recipient,
                    });
                }
                return Err(InternalError::invariant());
            }
            return Err(InternalError::conflict());
        }
        let mut asset = required_asset(canister_id)?;
        if !matches!(
            asset.status,
            CanisterPoolAssetStatusRecord::Ready | CanisterPoolAssetStatusRecord::Failed(_)
        ) {
            return Err(InternalError::conflict());
        }
        asset.status = CanisterPoolAssetStatusRecord::HandingOff { recipient };
        asset.updated_at_ns = prepared_at_ns;
        state.handoff = Some(CanisterPoolHandoffRecord {
            canister_id,
            recipient,
            prepared_at_ns,
        });
        CanisterPoolStore::insert(canister_id, asset);
        CanisterPoolStore::set_state(state);
        Ok(CanisterPoolHandoffView {
            canister_id,
            recipient,
        })
    }

    pub fn complete_handoff(
        canister_id: Principal,
        recipient: Principal,
        completed_at_ns: u64,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let handoff = state.handoff.ok_or_else(InternalError::unavailable)?;
        if handoff.canister_id != canister_id || handoff.recipient != recipient {
            return Err(InternalError::conflict());
        }
        let asset = required_asset(canister_id)?;
        if asset.status != (CanisterPoolAssetStatusRecord::HandingOff { recipient }) {
            return Err(InternalError::invariant());
        }
        if CanisterPoolStore::handoff_receipt(&canister_id).is_some() {
            return Err(InternalError::conflict());
        }
        CanisterPoolStore::remove(&canister_id);
        CanisterPoolStore::insert_handoff_receipt(
            canister_id,
            CanisterPoolHandoffReceiptRecord {
                recipient,
                completed_at_ns,
            },
        );
        state.handoff = None;
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    #[must_use]
    pub fn pending_handoff() -> Option<CanisterPoolHandoffView> {
        CanisterPoolStore::state()
            .handoff
            .map(|handoff| CanisterPoolHandoffView {
                canister_id: handoff.canister_id,
                recipient: handoff.recipient,
            })
    }

    /// Select the exact pending or next transferable non-Store asset during root draining.
    pub(crate) fn handoff_candidate() -> Option<Principal> {
        if let Some(pending) = Self::pending_handoff() {
            return Some(pending.canister_id);
        }
        CanisterPoolStore::export()
            .entries
            .into_iter()
            .find_map(|entry| {
                matches!(
                    entry.asset.status,
                    CanisterPoolAssetStatusRecord::Ready
                        | CanisterPoolAssetStatusRecord::Failed { .. }
                )
                .then_some(entry.canister_id)
            })
    }

    #[must_use]
    pub fn completed_handoff_recipient(canister_id: Principal) -> Option<Principal> {
        CanisterPoolStore::handoff_receipt(&canister_id).map(|receipt| receipt.recipient)
    }

    pub fn recycling_reset_is_terminal(canister_id: Principal) -> Result<bool, InternalError> {
        Ok(matches!(
            required_asset(canister_id)?.status,
            CanisterPoolAssetStatusRecord::Recycling {
                reset: CanisterPoolRecycleResetRecord::Ready
                    | CanisterPoolRecycleResetRecord::Failed(_),
                ..
            }
        ))
    }

    #[must_use]
    pub fn contains_asset(canister_id: Principal) -> bool {
        CanisterPoolStore::get(&canister_id).is_some()
    }

    #[must_use]
    pub fn ready_count() -> u32 {
        count_as_u32(
            CanisterPoolStore::export()
                .entries
                .into_iter()
                .filter(|entry| matches!(entry.asset.status, CanisterPoolAssetStatusRecord::Ready))
                .count(),
        )
    }

    /// Whether one Ready asset can satisfy the exact next Component demand.
    #[must_use]
    pub fn has_ready_asset_for(required_cycles: &Cycles) -> bool {
        CanisterPoolStore::export()
            .entries
            .into_iter()
            .any(|entry| {
                matches!(entry.asset.status, CanisterPoolAssetStatusRecord::Ready)
                    && entry.asset.cycles >= *required_cycles
            })
    }

    /// Whether distinct Ready assets cover every exact demand without double assignment.
    #[must_use]
    pub fn ready_assets_cover(required_cycles: &[Cycles]) -> bool {
        let mut ready = CanisterPoolStore::export()
            .entries
            .into_iter()
            .filter(|entry| matches!(entry.asset.status, CanisterPoolAssetStatusRecord::Ready))
            .map(|entry| entry.asset.cycles)
            .collect::<Vec<_>>();
        ready.sort();
        let mut required = required_cycles.to_vec();
        required.sort();
        let mut ready = ready.into_iter();
        let mut candidate = ready.next();
        for demand in required {
            while candidate.as_ref().is_some_and(|cycles| cycles < &demand) {
                candidate = ready.next();
            }
            if candidate.is_none() {
                return false;
            }
            candidate = ready.next();
        }
        true
    }

    #[must_use]
    pub fn pooled_asset_count() -> u32 {
        count_as_u32(
            CanisterPoolStore::export()
                .entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.asset.status,
                        CanisterPoolAssetStatusRecord::PendingReset
                            | CanisterPoolAssetStatusRecord::Ready
                            | CanisterPoolAssetStatusRecord::HandingOff { .. }
                            | CanisterPoolAssetStatusRecord::Failed(_)
                    )
                })
                .count(),
        )
    }

    #[must_use]
    pub fn standby_capacity_is_exhausted(config: &FleetSubnetCanisterPoolConfig) -> bool {
        let pending_creation = u64::from(CanisterPoolStore::state().creation.is_some());
        u64::from(Self::pooled_asset_count()) + pending_creation >= u64::from(config.maximum_size)
    }

    /// Return every pool-side physical asset represented in a compact root summary.
    ///
    /// A claimed asset remains pool-side until its Component Registry principal is
    /// durably committed and the claim is finalized as a workload.
    #[must_use]
    pub fn summary_pool_asset_count() -> u32 {
        count_as_u32(
            CanisterPoolStore::export()
                .entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.asset.status,
                        CanisterPoolAssetStatusRecord::PendingReset
                            | CanisterPoolAssetStatusRecord::Ready
                            | CanisterPoolAssetStatusRecord::Claimed(_)
                            | CanisterPoolAssetStatusRecord::HandingOff { .. }
                            | CanisterPoolAssetStatusRecord::Failed(_)
                    )
                })
                .count(),
        )
    }

    #[must_use]
    pub fn workload_count() -> u32 {
        count_as_u32(
            CanisterPoolStore::export()
                .entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.asset.status,
                        CanisterPoolAssetStatusRecord::Workload(_)
                            | CanisterPoolAssetStatusRecord::Recycling { .. }
                    )
                })
                .count(),
        )
    }

    #[must_use]
    pub fn store_count() -> u32 {
        count_as_u32(
            CanisterPoolStore::export()
                .entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.asset.status,
                        CanisterPoolAssetStatusRecord::Store
                            | CanisterPoolAssetStatusRecord::StoreDeletionPending { .. }
                    )
                })
                .count(),
        )
    }

    #[must_use]
    pub fn non_store_asset_count() -> u32 {
        count_as_u32(
            CanisterPoolStore::export()
                .entries
                .into_iter()
                .filter(|entry| {
                    !matches!(
                        entry.asset.status,
                        CanisterPoolAssetStatusRecord::Store
                            | CanisterPoolAssetStatusRecord::StoreDeletionPending { .. }
                    )
                })
                .count(),
        )
    }

    pub fn complete_recycling(
        canister_id: Principal,
        component: ComponentInstanceId,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        let Some((claim, next_status)) = recycling_completion(&asset, component)? else {
            return Ok(());
        };
        asset.status = next_status;
        asset.last_recycle = Some(claim);
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
    }

    /// Prove that Registry membership removal can deterministically settle the
    /// physical recycling row without mutating either authority.
    pub fn validate_complete_recycling(
        canister_id: Principal,
        component: ComponentInstanceId,
    ) -> Result<(), InternalError> {
        recycling_completion(&required_asset(canister_id)?, component).map(|_| ())
    }

    pub fn require_store(canister_id: Principal) -> Result<(), InternalError> {
        let asset = required_asset(canister_id)?;
        if asset.origin != CanisterPoolAssetOriginRecord::InfrastructureStore
            || !matches!(
                asset.status,
                CanisterPoolAssetStatusRecord::Store
                    | CanisterPoolAssetStatusRecord::StoreDeletionPending { .. }
            )
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    pub fn begin_store_deletion(
        canister_id: Principal,
        operation_id: [u8; 32],
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        if asset.origin != CanisterPoolAssetOriginRecord::InfrastructureStore {
            return Err(InternalError::conflict());
        }
        match asset.status {
            CanisterPoolAssetStatusRecord::Store => {
                asset.status = CanisterPoolAssetStatusRecord::StoreDeletionPending { operation_id };
                asset.updated_at_ns = now_ns;
                CanisterPoolStore::insert(canister_id, asset);
                Ok(())
            }
            CanisterPoolAssetStatusRecord::StoreDeletionPending {
                operation_id: existing,
            } if existing == operation_id => Ok(()),
            _ => Err(InternalError::conflict()),
        }
    }

    pub fn complete_store_deletion(
        canister_id: Principal,
        operation_id: [u8; 32],
    ) -> Result<(), InternalError> {
        let Some(asset) = CanisterPoolStore::get(&canister_id) else {
            return Ok(());
        };
        if asset.origin != CanisterPoolAssetOriginRecord::InfrastructureStore
            || asset.status
                != (CanisterPoolAssetStatusRecord::StoreDeletionPending { operation_id })
        {
            return Err(InternalError::conflict());
        }
        CanisterPoolStore::remove(&canister_id);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        CanisterPoolStore::clear();
    }
}

const fn validate_config(config: &FleetSubnetCanisterPoolConfig) -> Result<(), InternalError> {
    if config.minimum_size == 0 {
        return Err(InternalError::invalid_input());
    }
    if config.maximum_size < config.minimum_size {
        return Err(InternalError::invalid_input());
    }
    if config.canister_cycles.to_u128() == 0 {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn creation_authority(existing: &CanisterPoolCreationRecord) -> CanisterPoolCreationAuthority {
    CanisterPoolCreationAuthority {
        operation_id: existing.operation_id,
        cycles_ledger: existing.cycles_ledger,
        placement_subnet: existing.placement_subnet,
        root: existing.root,
        ledger_amount: existing.ledger_amount.clone(),
        created_at_time_ns: existing.created_at_time_ns,
    }
}

fn require_creation_attempt(
    creation: &CanisterPoolCreationRecord,
    operation_id: [u8; 32],
    settlement: ReplayCostGuardSettlement,
) -> Result<(), InternalError> {
    require_creation_operation(creation, operation_id)?;
    if creation.cost_guard_settlement != Some(settlement) {
        return Err(InternalError::conflict());
    }
    if !matches!(
        creation.progress,
        CanisterPoolCreationProgressRecord::Intent { .. }
    ) {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn require_creation_operation(
    creation: &CanisterPoolCreationRecord,
    operation_id: [u8; 32],
) -> Result<(), InternalError> {
    if creation.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    Ok(())
}

const fn require_creation_cost_settled(
    creation: &CanisterPoolCreationRecord,
) -> Result<(), InternalError> {
    if creation.cost_guard_settlement.is_some() {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn require_created_inventory_adoption(canister_id: Principal) -> Result<(), InternalError> {
    let adopted =
        CanisterPoolStore::get(&canister_id).is_some_and(|asset| created_asset_is_adopted(&asset));
    if !adopted {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn created_asset_is_adopted(asset: &CanisterPoolAssetRecord) -> bool {
    if asset.origin != CanisterPoolAssetOriginRecord::Created {
        return false;
    }
    asset.status == CanisterPoolAssetStatusRecord::PendingReset
}

const fn creation_is_known_unapplied(creation: &CanisterPoolCreationRecord) -> bool {
    matches!(
        creation.progress,
        CanisterPoolCreationProgressRecord::Intent {
            uncertain_result: false
        } | CanisterPoolCreationProgressRecord::Blocked {
            failure: CanisterPoolCreationFailureRecord::LedgerCreationFailed
                | CanisterPoolCreationFailureRecord::LedgerRejected
        }
    )
}

const fn creation_is_known_unapplied_intent(creation: &CanisterPoolCreationRecord) -> bool {
    matches!(
        creation.progress,
        CanisterPoolCreationProgressRecord::Intent {
            uncertain_result: false
        }
    )
}

fn validate_new_asset_capacity(
    config: &FleetSubnetCanisterPoolConfig,
    canister_id: Principal,
) -> Result<(), InternalError> {
    validate_config(config)?;
    if CanisterPoolStore::get(&canister_id).is_some() {
        return Ok(());
    }
    if CanisterPoolOps::standby_capacity_is_exhausted(config) {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}

fn required_asset(canister_id: Principal) -> Result<CanisterPoolAssetRecord, InternalError> {
    CanisterPoolStore::get(&canister_id).ok_or_else(InternalError::unavailable)
}

fn recycling_completion(
    asset: &CanisterPoolAssetRecord,
    component: ComponentInstanceId,
) -> Result<Option<(CanisterPoolClaimRecord, CanisterPoolAssetStatusRecord)>, InternalError> {
    if asset
        .last_recycle
        .as_ref()
        .is_some_and(|claim| claim.component == component)
    {
        return Ok(None);
    }
    let CanisterPoolAssetStatusRecord::Recycling { claim, reset } = &asset.status else {
        return Err(InternalError::conflict());
    };
    if claim.component != component {
        return Err(InternalError::conflict());
    }
    let next_status = match reset {
        CanisterPoolRecycleResetRecord::Ready => CanisterPoolAssetStatusRecord::Ready,
        CanisterPoolRecycleResetRecord::Failed(reason) => {
            CanisterPoolAssetStatusRecord::Failed(reason.clone())
        }
        CanisterPoolRecycleResetRecord::Pending => {
            return Err(InternalError::unavailable());
        }
    };
    Ok(Some((claim.clone(), next_status)))
}

const fn claim_record(claim: &CanisterPoolClaimKey) -> CanisterPoolClaimRecord {
    CanisterPoolClaimRecord {
        component: claim.component,
        operation_id: claim.operation_id,
    }
}

fn validate_ledger_recovery_request(
    request: &PoolLedgerRecoveryRequest,
) -> Result<(), InternalError> {
    let balance = request.ledger_balance.to_u128();
    let fee = request.ledger_fee.to_u128();
    let expected = balance
        .checked_sub(fee)
        .filter(|amount| *amount > 0)
        .ok_or_else(InternalError::invalid_input)?;
    if request.operation_id == [0; 32]
        || request.canister_id == Principal::anonymous()
        || request.cycles_ledger == Principal::anonymous()
        || request.created_at_time_ns == 0
        || request.maximum_execution_burn_cycles.to_u128() == 0
        || request.withdrawal_amount.to_u128() != expected
        || request.artifact.payload_size_bytes == 0
        || request.artifact.payload_hash == [0; 32]
        || request.artifact.raw_module_hash == [0; 32]
        || request.artifact.candid_sha256 == [0; 32]
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn ledger_recovery_authority_from_dto(
    request: &PoolLedgerRecoveryRequest,
) -> CanisterPoolLedgerRecoveryAuthorityRecord {
    CanisterPoolLedgerRecoveryAuthorityRecord {
        artifact: CanisterPoolLedgerRecoveryArtifactRecord {
            candid_sha256: request.artifact.candid_sha256,
            payload_hash: request.artifact.payload_hash,
            payload_size_bytes: request.artifact.payload_size_bytes,
            raw_module_hash: request.artifact.raw_module_hash,
            release_build_id: request.artifact.release_build_id,
        },
        canister_id: request.canister_id,
        created_at_time_ns: request.created_at_time_ns,
        cycles_ledger: request.cycles_ledger,
        ledger_balance: request.ledger_balance.clone(),
        ledger_fee: request.ledger_fee.clone(),
        maximum_execution_burn_cycles: request.maximum_execution_burn_cycles.clone(),
        operation_id: request.operation_id,
        withdrawal_amount: request.withdrawal_amount.clone(),
    }
}

fn ledger_recovery_request_to_dto(
    authority: &CanisterPoolLedgerRecoveryAuthorityRecord,
) -> PoolLedgerRecoveryRequest {
    PoolLedgerRecoveryRequest {
        artifact: PoolLedgerRecoveryArtifact {
            candid_sha256: authority.artifact.candid_sha256,
            payload_hash: authority.artifact.payload_hash,
            payload_size_bytes: authority.artifact.payload_size_bytes,
            raw_module_hash: authority.artifact.raw_module_hash,
            release_build_id: authority.artifact.release_build_id,
        },
        canister_id: authority.canister_id,
        created_at_time_ns: authority.created_at_time_ns,
        cycles_ledger: authority.cycles_ledger,
        ledger_balance: authority.ledger_balance.clone(),
        ledger_fee: authority.ledger_fee.clone(),
        maximum_execution_burn_cycles: authority.maximum_execution_burn_cycles.clone(),
        operation_id: authority.operation_id,
        withdrawal_amount: authority.withdrawal_amount.clone(),
    }
}

fn ledger_recovery_status(
    current: &CanisterPoolLedgerRecoveryRecord,
) -> PoolLedgerRecoveryStatusResponse {
    let (phase, block_index) = match current.phase {
        CanisterPoolLedgerRecoveryPhaseRecord::Prepared => {
            (PoolLedgerRecoveryPhase::Prepared, None)
        }
        CanisterPoolLedgerRecoveryPhaseRecord::HelperInstallIssued => {
            (PoolLedgerRecoveryPhase::HelperInstallIssued, None)
        }
        CanisterPoolLedgerRecoveryPhaseRecord::HelperInstalled => {
            (PoolLedgerRecoveryPhase::HelperInstalled, None)
        }
        CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalIssued => {
            (PoolLedgerRecoveryPhase::WithdrawalIssued, None)
        }
        CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalVerified { block_index } => (
            PoolLedgerRecoveryPhase::WithdrawalVerified,
            Some(block_index),
        ),
        CanisterPoolLedgerRecoveryPhaseRecord::HelperUninstallIssued { block_index } => (
            PoolLedgerRecoveryPhase::HelperUninstallIssued,
            Some(block_index),
        ),
    };
    PoolLedgerRecoveryStatusResponse {
        block_index,
        initial_native_cycles: current.initial_native_cycles.clone(),
        phase,
        receipt: None,
        request: ledger_recovery_request_to_dto(&current.authority),
    }
}

fn ledger_recovery_receipt_status(
    receipt: &CanisterPoolLedgerRecoveryReceiptRecord,
) -> PoolLedgerRecoveryStatusResponse {
    PoolLedgerRecoveryStatusResponse {
        block_index: Some(receipt.block_index),
        initial_native_cycles: receipt.initial_native_cycles.clone(),
        phase: PoolLedgerRecoveryPhase::Complete,
        receipt: Some(ledger_recovery_receipt_to_dto(receipt)),
        request: ledger_recovery_request_to_dto(&receipt.authority),
    }
}

fn ledger_recovery_receipt_to_dto(
    receipt: &CanisterPoolLedgerRecoveryReceiptRecord,
) -> PoolLedgerRecoveryReceipt {
    PoolLedgerRecoveryReceipt {
        block_index: receipt.block_index,
        completed_at_ns: receipt.completed_at_ns,
        final_native_cycles: receipt.final_native_cycles.clone(),
        operation_id: receipt.authority.operation_id,
        request: ledger_recovery_request_to_dto(&receipt.authority),
    }
}

const fn transition_matches(
    existing: CanisterPoolLedgerRecoveryPhaseRecord,
    requested: CanisterPoolLedgerRecoveryTransition,
) -> bool {
    match (existing, requested) {
        (
            CanisterPoolLedgerRecoveryPhaseRecord::HelperInstallIssued,
            CanisterPoolLedgerRecoveryTransition::HelperInstallIssued,
        )
        | (
            CanisterPoolLedgerRecoveryPhaseRecord::HelperInstalled,
            CanisterPoolLedgerRecoveryTransition::HelperInstalled,
        )
        | (
            CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalIssued,
            CanisterPoolLedgerRecoveryTransition::WithdrawalIssued,
        ) => true,
        (
            CanisterPoolLedgerRecoveryPhaseRecord::WithdrawalVerified {
                block_index: existing,
            },
            CanisterPoolLedgerRecoveryTransition::WithdrawalVerified {
                block_index: requested,
            },
        )
        | (
            CanisterPoolLedgerRecoveryPhaseRecord::HelperUninstallIssued {
                block_index: existing,
            },
            CanisterPoolLedgerRecoveryTransition::HelperUninstallIssued {
                block_index: requested,
            },
        ) => existing == requested,
        _ => false,
    }
}

fn count_as_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

const fn creation_to_dto(creation: CanisterPoolCreationRecord) -> CanisterPoolCreation {
    let progress = match creation.progress {
        CanisterPoolCreationProgressRecord::Intent { uncertain_result } => {
            CanisterPoolCreationProgress::Intent { uncertain_result }
        }
        CanisterPoolCreationProgressRecord::Created {
            block_index,
            canister_id,
        } => CanisterPoolCreationProgress::Created {
            block_index,
            canister_id,
        },
        CanisterPoolCreationProgressRecord::Blocked { failure } => {
            CanisterPoolCreationProgress::Blocked {
                failure: creation_failure_to_dto(failure),
            }
        }
    };
    CanisterPoolCreation {
        operation_id: creation.operation_id,
        cycles_ledger: creation.cycles_ledger,
        placement_subnet: creation.placement_subnet,
        root: creation.root,
        ledger_amount: creation.ledger_amount,
        created_at_time_ns: creation.created_at_time_ns,
        progress,
    }
}

const fn creation_failure_to_dto(
    failure: CanisterPoolCreationFailureRecord,
) -> CanisterPoolCreationFailure {
    match failure {
        CanisterPoolCreationFailureRecord::UnresolvedAfterLedgerWindow => {
            CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow
        }
        CanisterPoolCreationFailureRecord::LedgerCreationFailed => {
            CanisterPoolCreationFailure::LedgerCreationFailed
        }
        CanisterPoolCreationFailureRecord::LedgerRejected => {
            CanisterPoolCreationFailure::LedgerRejected
        }
    }
}

pub const fn creation_failure_view_to_dto(
    failure: CanisterPoolCreationFailureView,
) -> CanisterPoolCreationFailure {
    match failure {
        CanisterPoolCreationFailureView::UnresolvedAfterLedgerWindow => {
            CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow
        }
        CanisterPoolCreationFailureView::LedgerCreationFailed => {
            CanisterPoolCreationFailure::LedgerCreationFailed
        }
        CanisterPoolCreationFailureView::LedgerRejected => {
            CanisterPoolCreationFailure::LedgerRejected
        }
    }
}

const fn creation_failure_to_view(
    failure: CanisterPoolCreationFailureRecord,
) -> CanisterPoolCreationFailureView {
    match failure {
        CanisterPoolCreationFailureRecord::UnresolvedAfterLedgerWindow => {
            CanisterPoolCreationFailureView::UnresolvedAfterLedgerWindow
        }
        CanisterPoolCreationFailureRecord::LedgerCreationFailed => {
            CanisterPoolCreationFailureView::LedgerCreationFailed
        }
        CanisterPoolCreationFailureRecord::LedgerRejected => {
            CanisterPoolCreationFailureView::LedgerRejected
        }
    }
}

const fn creation_failure_from_dto(
    failure: CanisterPoolCreationFailure,
) -> CanisterPoolCreationFailureRecord {
    match failure {
        CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow => {
            CanisterPoolCreationFailureRecord::UnresolvedAfterLedgerWindow
        }
        CanisterPoolCreationFailure::LedgerCreationFailed => {
            CanisterPoolCreationFailureRecord::LedgerCreationFailed
        }
        CanisterPoolCreationFailure::LedgerRejected => {
            CanisterPoolCreationFailureRecord::LedgerRejected
        }
    }
}

fn asset_to_dto(canister_id: Principal, asset: CanisterPoolAssetRecord) -> CanisterPoolAsset {
    CanisterPoolAsset {
        canister_id,
        cycles: asset.cycles,
        origin: match asset.origin {
            CanisterPoolAssetOriginRecord::InfrastructureStore => {
                CanisterPoolAssetOrigin::InfrastructureStore
            }
            CanisterPoolAssetOriginRecord::Created => CanisterPoolAssetOrigin::Created,
            CanisterPoolAssetOriginRecord::Imported => CanisterPoolAssetOrigin::Imported,
            CanisterPoolAssetOriginRecord::Recycled => CanisterPoolAssetOrigin::Recycled,
        },
        status: match asset.status {
            CanisterPoolAssetStatusRecord::Store => CanisterPoolAssetStatus::Store,
            CanisterPoolAssetStatusRecord::StoreDeletionPending { operation_id } => {
                CanisterPoolAssetStatus::StoreDeletionPending { operation_id }
            }
            CanisterPoolAssetStatusRecord::PendingReset => CanisterPoolAssetStatus::PendingReset,
            CanisterPoolAssetStatusRecord::Ready => CanisterPoolAssetStatus::Ready,
            CanisterPoolAssetStatusRecord::Claimed(claim) => CanisterPoolAssetStatus::Claimed {
                claim: CanisterPoolClaim {
                    component: claim.component,
                    operation_id: claim.operation_id,
                },
            },
            CanisterPoolAssetStatusRecord::Workload(claim) => CanisterPoolAssetStatus::Workload {
                claim: CanisterPoolClaim {
                    component: claim.component,
                    operation_id: claim.operation_id,
                },
            },
            CanisterPoolAssetStatusRecord::Recycling { claim, reset } => {
                CanisterPoolAssetStatus::Recycling {
                    claim: CanisterPoolClaim {
                        component: claim.component,
                        operation_id: claim.operation_id,
                    },
                    reset: match reset {
                        CanisterPoolRecycleResetRecord::Pending => {
                            CanisterPoolRecycleReset::Pending
                        }
                        CanisterPoolRecycleResetRecord::Ready => CanisterPoolRecycleReset::Ready,
                        CanisterPoolRecycleResetRecord::Failed(reason) => {
                            CanisterPoolRecycleReset::Failed { reason }
                        }
                    },
                }
            }
            CanisterPoolAssetStatusRecord::RecoveringLedger { operation_id } => {
                CanisterPoolAssetStatus::RecoveringLedger { operation_id }
            }
            CanisterPoolAssetStatusRecord::HandingOff { recipient } => {
                CanisterPoolAssetStatus::HandingOff { recipient }
            }
            CanisterPoolAssetStatusRecord::Failed(reason) => {
                CanisterPoolAssetStatus::Failed { reason }
            }
        },
        added_at_ns: asset.added_at_ns,
        updated_at_ns: asset.updated_at_ns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::IntentId;

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    fn ledger_recovery_request(canister_id: Principal) -> PoolLedgerRecoveryRequest {
        PoolLedgerRecoveryRequest {
            artifact: PoolLedgerRecoveryArtifact {
                candid_sha256: [1; 32],
                payload_hash: [2; 32],
                payload_size_bytes: 123,
                raw_module_hash: [3; 32],
                release_build_id: "44".repeat(32).parse().expect("release build ID"),
            },
            canister_id,
            created_at_time_ns: 9,
            cycles_ledger: principal(90),
            ledger_balance: Cycles::new(1_000),
            ledger_fee: Cycles::new(10),
            maximum_execution_burn_cycles: Cycles::new(20),
            operation_id: [5; 32],
            withdrawal_amount: Cycles::new(990),
        }
    }

    #[test]
    fn ledger_recovery_fences_one_empty_asset_and_replays_only_exact_authority() {
        CanisterPoolStore::clear();
        let canister_id = principal(91);
        imported_ready(canister_id, Cycles::new(2_000), 1);
        let request = ledger_recovery_request(canister_id);

        let prepared = CanisterPoolOps::prepare_ledger_recovery(&request, Cycles::new(2_000), 2)
            .expect("prepare exact recovery");
        assert_eq!(prepared.phase, PoolLedgerRecoveryPhase::Prepared);
        assert_eq!(CanisterPoolOps::ready_count(), 0);
        assert!(CanisterPoolOps::has_pending_lifecycle_work());
        assert!(CanisterPoolOps::has_pending_ledger_recovery());
        assert_eq!(
            CanisterPoolOps::response(config(), None, 10).recovering_ledger,
            1
        );
        assert_eq!(
            CanisterPoolOps::prepare_ledger_recovery(&request, Cycles::new(2_000), 3)
                .expect("exact prepare replay"),
            prepared
        );
        let mut conflicting = request.clone();
        conflicting.withdrawal_amount = Cycles::new(989);
        assert!(
            CanisterPoolOps::prepare_ledger_recovery(&conflicting, Cycles::new(2_000), 3,).is_err()
        );

        for transition in [
            CanisterPoolLedgerRecoveryTransition::HelperInstallIssued,
            CanisterPoolLedgerRecoveryTransition::HelperInstalled,
            CanisterPoolLedgerRecoveryTransition::WithdrawalIssued,
            CanisterPoolLedgerRecoveryTransition::WithdrawalVerified { block_index: 7 },
            CanisterPoolLedgerRecoveryTransition::HelperUninstallIssued { block_index: 7 },
        ] {
            CanisterPoolOps::advance_ledger_recovery(&request, transition)
                .expect("advance exact recovery");
            CanisterPoolOps::advance_ledger_recovery(&request, transition)
                .expect("exact phase replay");
        }
        assert!(
            CanisterPoolOps::advance_ledger_recovery(
                &request,
                CanisterPoolLedgerRecoveryTransition::HelperUninstallIssued { block_index: 8 },
            )
            .is_err()
        );
        let receipt = CanisterPoolOps::complete_ledger_recovery(&request, Cycles::new(2_970), 4)
            .expect("complete exact recovery");
        assert_eq!(receipt.block_index, 7);
        assert_eq!(
            CanisterPoolOps::complete_ledger_recovery(&request, Cycles::new(2_970), 5)
                .expect("terminal exact replay"),
            receipt
        );
        assert_eq!(CanisterPoolOps::ready_count(), 1);
        assert!(!CanisterPoolOps::has_pending_lifecycle_work());
        assert!(!CanisterPoolOps::has_pending_ledger_recovery());
        let mut terminal_conflict = request.clone();
        terminal_conflict.maximum_execution_burn_cycles = Cycles::new(21);
        assert!(
            CanisterPoolOps::prepare_ledger_recovery(&terminal_conflict, Cycles::new(2_970), 6,)
                .is_err()
        );
        let retained = CanisterPoolOps::ledger_recovery_status_by_operation(request.operation_id)
            .expect("first terminal receipt survives conflicting authority");
        assert_eq!(retained.phase, PoolLedgerRecoveryPhase::Complete);
        assert_eq!(retained.receipt, Some(receipt.clone()));
        assert_eq!(retained.request, request);

        let second_canister_id = principal(92);
        imported_ready(second_canister_id, Cycles::new(4_000), 6);
        let mut second_request = ledger_recovery_request(second_canister_id);
        second_request.created_at_time_ns = 10;
        second_request.operation_id = [6; 32];

        CanisterPoolOps::prepare_ledger_recovery(&second_request, Cycles::new(4_000), 7)
            .expect("prepare distinct second recovery after terminal first receipt");
        for transition in [
            CanisterPoolLedgerRecoveryTransition::HelperInstallIssued,
            CanisterPoolLedgerRecoveryTransition::HelperInstalled,
            CanisterPoolLedgerRecoveryTransition::WithdrawalIssued,
            CanisterPoolLedgerRecoveryTransition::WithdrawalVerified { block_index: 8 },
            CanisterPoolLedgerRecoveryTransition::HelperUninstallIssued { block_index: 8 },
        ] {
            CanisterPoolOps::advance_ledger_recovery(&second_request, transition)
                .expect("advance distinct second recovery");
        }
        let second_receipt =
            CanisterPoolOps::complete_ledger_recovery(&second_request, Cycles::new(4_970), 8)
                .expect("complete distinct second recovery");
        assert_eq!(second_receipt.block_index, 8);
        assert_eq!(
            CanisterPoolOps::complete_ledger_recovery(&second_request, Cycles::new(4_970), 9,)
                .expect("replay distinct second terminal receipt"),
            second_receipt
        );
        assert_eq!(CanisterPoolOps::ready_count(), 2);
        assert!(!CanisterPoolOps::has_pending_lifecycle_work());
        assert!(!CanisterPoolOps::has_pending_ledger_recovery());
        CanisterPoolStore::clear();
    }

    fn config() -> FleetSubnetCanisterPoolConfig {
        FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 4,
            canister_cycles: Cycles::new(100),
        }
    }

    fn imported_ready(canister_id: Principal, cycles: Cycles, now_ns: u64) {
        CanisterPoolOps::initialize_imports(&config(), &[canister_id], now_ns)
            .expect("import asset");
        CanisterPoolOps::mark_ready(canister_id, cycles, now_ns).expect("ready asset");
    }

    fn creation_authority_for(operation_id: [u8; 32]) -> CanisterPoolCreationAuthority {
        CanisterPoolCreationAuthority {
            operation_id,
            cycles_ledger: principal(8),
            placement_subnet: principal(7),
            root: principal(6),
            ledger_amount: Cycles::new(1_000),
            created_at_time_ns: 10,
        }
    }

    #[test]
    fn ledger_creation_adopts_exact_principal_before_advancing_sequence() {
        CanisterPoolStore::clear();
        let operation_id = [9; 32];
        let settlement = ReplayCostGuardSettlement {
            quota_intent_id: IntentId(1),
            reservation_intent_id: IntentId(2),
        };
        CanisterPoolOps::begin_creation(creation_authority_for(operation_id), 10)
            .expect("begin creation");
        CanisterPoolOps::begin_creation(creation_authority_for(operation_id), 11)
            .expect("exact creation replay");
        CanisterPoolOps::begin_creation_attempt(operation_id, settlement)
            .expect("begin ledger attempt");
        let created = principal(5);
        CanisterPoolOps::mark_creation_created(operation_id, 12, created)
            .expect("record ledger receipt");
        CanisterPoolOps::register_created_pending_reset(operation_id, created, 13)
            .expect("adopt created principal");
        assert!(CanisterPoolOps::commit_creation(operation_id).is_err());
        CanisterPoolOps::settle_creation_attempt(operation_id, settlement)
            .expect("settle attempt authority");
        CanisterPoolOps::commit_creation(operation_id).expect("commit refill");

        assert_eq!(CanisterPoolOps::next_creation_sequence(), 1);
        assert_eq!(
            CanisterPoolOps::next_creation_timestamp(9).expect("next creation timestamp"),
            11
        );
        assert_eq!(CanisterPoolOps::pending_creation(), None);
        let asset = CanisterPoolStore::get(&created).expect("created physical inventory row");
        assert_eq!(asset.origin, CanisterPoolAssetOriginRecord::Created);
        assert_eq!(asset.status, CanisterPoolAssetStatusRecord::PendingReset);

        let next_operation_id = [8; 32];
        assert!(
            CanisterPoolOps::begin_creation(creation_authority_for(next_operation_id), 14).is_err()
        );
        let mut next = creation_authority_for(next_operation_id);
        next.created_at_time_ns = 11;
        CanisterPoolOps::begin_creation(next, 14).expect("begin monotonic refill");
        CanisterPoolStore::clear();
    }

    #[test]
    fn draining_cancels_only_known_unapplied_creation() {
        CanisterPoolStore::clear();
        let known_operation_id = [7; 32];
        CanisterPoolOps::begin_creation(creation_authority_for(known_operation_id), 10)
            .expect("begin known creation");
        let cancelled = CanisterPoolOps::cancel_known_unapplied_creation()
            .expect("cancel known-unapplied creation");
        assert_eq!(cancelled, known_operation_id);
        assert_eq!(CanisterPoolOps::next_creation_sequence(), 1);

        let uncertain_operation_id = [6; 32];
        let mut uncertain = creation_authority_for(uncertain_operation_id);
        uncertain.created_at_time_ns = 11;
        CanisterPoolOps::begin_creation(uncertain, 11).expect("begin uncertain creation");
        let settlement = ReplayCostGuardSettlement {
            quota_intent_id: IntentId(3),
            reservation_intent_id: IntentId(4),
        };
        CanisterPoolOps::begin_creation_attempt(uncertain_operation_id, settlement)
            .expect("begin uncertain attempt");
        assert!(CanisterPoolOps::cancel_known_unapplied_creation().is_err());
        CanisterPoolStore::clear();
    }

    #[test]
    fn pending_creation_reserves_one_standby_capacity_slot() {
        CanisterPoolStore::clear();
        imported_ready(principal(1), Cycles::new(100), 1);
        imported_ready(principal(2), Cycles::new(100), 2);
        imported_ready(principal(3), Cycles::new(100), 3);
        CanisterPoolOps::begin_creation(creation_authority_for([5; 32]), 10)
            .expect("begin capacity-reserving creation");

        assert!(CanisterPoolOps::standby_capacity_is_exhausted(&config()));
        assert!(CanisterPoolOps::initialize_imports(&config(), &[principal(4)], 11).is_err());
        CanisterPoolStore::clear();
    }

    #[test]
    fn equal_capacity_claim_is_oldest_first_and_exactly_replayable() {
        CanisterPoolStore::clear();
        imported_ready(principal(2), Cycles::new(100), 20);
        imported_ready(principal(1), Cycles::new(100), 10);
        let claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([7; 32]),
            operation_id: [7; 32],
        };

        let selected =
            CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(100), 30)
                .expect("claim")
                .expect("ready asset");
        assert_eq!(selected, principal(1));
        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(100), 40)
                .expect("replay claim"),
            Some(selected)
        );

        CanisterPoolOps::finalize_claim(&claim, selected, 50).expect("finalize claim");
        assert_eq!(CanisterPoolOps::pooled_asset_count(), 1);
        assert_eq!(CanisterPoolOps::summary_pool_asset_count(), 1);
        assert_eq!(CanisterPoolOps::workload_count(), 1);
        CanisterPoolStore::clear();
    }

    #[test]
    fn store_pool_and_workload_are_exclusive_physical_inventory_states() {
        CanisterPoolStore::clear();
        let store = principal(9);
        let workload = principal(1);
        let component = ComponentInstanceId::from_generated_bytes([3; 32]);
        let operation_id = [4; 32];
        CanisterPoolOps::initialize_store(store, 1).expect("initialize sibling Store");
        imported_ready(workload, Cycles::new(100), 2);
        let claim = CanisterPoolClaimKey {
            component,
            operation_id,
        };
        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(100), 3)
                .expect("claim pool asset"),
            Some(workload)
        );
        CanisterPoolOps::finalize_claim(&claim, workload, 4).expect("commit workload state");

        let active = CanisterPoolOps::response(config(), None, 10);
        assert_eq!(active.tracked, 2);
        assert_eq!(active.store, 1);
        assert_eq!(active.store_deletion_pending, 0);
        assert_eq!(active.pooled, 0);
        assert_eq!(active.workload, 1);
        assert_eq!(CanisterPoolOps::store_count(), 1);
        assert_eq!(CanisterPoolOps::workload_count(), 1);

        CanisterPoolOps::begin_store_deletion(store, [5; 32], 5).expect("fence Store deletion");
        let deleting = CanisterPoolOps::response(config(), None, 10);
        assert_eq!(deleting.store, 0);
        assert_eq!(deleting.store_deletion_pending, 1);
        assert_eq!(deleting.workload, 1);
        assert!(CanisterPoolOps::mark_failed(store, None, "wrong state".to_string(), 6).is_err());

        CanisterPoolOps::complete_store_deletion(store, [5; 32])
            .expect("remove terminally deleted Store");
        let deleted = CanisterPoolOps::response(config(), None, 10);
        assert_eq!(deleted.tracked, 1);
        assert_eq!(deleted.store, 0);
        assert_eq!(deleted.store_deletion_pending, 0);
        assert_eq!(deleted.workload, 1);
        CanisterPoolStore::clear();
    }

    #[test]
    fn insufficient_cycle_assets_are_not_claimed() {
        CanisterPoolStore::clear();
        imported_ready(principal(1), Cycles::new(99), 10);
        let claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([8; 32]),
            operation_id: [8; 32],
        };

        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(100), 20)
                .expect("claim decision"),
            None
        );
        CanisterPoolStore::clear();
    }

    #[test]
    fn ready_capacity_matches_distinct_assets_to_exact_cycle_demands() {
        CanisterPoolStore::clear();
        imported_ready(principal(1), Cycles::new(20), 10);
        imported_ready(principal(2), Cycles::new(45), 20);

        assert!(CanisterPoolOps::ready_assets_cover(&[
            Cycles::new(40),
            Cycles::new(20),
        ]));
        assert!(!CanisterPoolOps::ready_assets_cover(&[
            Cycles::new(20),
            Cycles::new(50),
        ]));
        assert!(!CanisterPoolOps::has_ready_asset_for(&Cycles::new(50)));
        CanisterPoolStore::clear();
    }

    #[test]
    fn claims_preserve_heterogeneous_capacity_for_later_larger_demand() {
        CanisterPoolStore::clear();
        let large = principal(1);
        let small = principal(2);
        imported_ready(large, Cycles::new(50), 10);
        imported_ready(small, Cycles::new(20), 20);
        let first_claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([8; 32]),
            operation_id: [8; 32],
        };
        let second_claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([9; 32]),
            operation_id: [9; 32],
        };

        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&first_claim, &Cycles::new(20), 30,)
                .expect("claim smaller demand"),
            Some(small)
        );
        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&second_claim, &Cycles::new(50), 40,)
                .expect("claim later larger demand"),
            Some(large)
        );
        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&first_claim, &Cycles::new(20), 50,)
                .expect("replay smaller claim"),
            Some(small)
        );
        CanisterPoolStore::clear();
    }

    #[test]
    fn undersized_ready_import_can_be_queued_for_balance_reinspection() {
        CanisterPoolStore::clear();
        let canister_id = principal(1);
        imported_ready(canister_id, Cycles::new(45), 10);

        CanisterPoolOps::retry_reset(canister_id, &Cycles::new(50), 20)
            .expect("queue undersized Ready asset");
        CanisterPoolOps::retry_reset(canister_id, &Cycles::new(50), 21)
            .expect("exact queued retry");
        assert_eq!(
            CanisterPoolOps::prepare_ready_reinspection(canister_id, &Cycles::new(50), 22)
                .expect("resume the fenced Ready-row reinspection"),
            CanisterPoolResetPreparation::Reinspect
        );
        assert_eq!(
            CanisterPoolOps::pending_reset_canisters(),
            vec![canister_id]
        );

        CanisterPoolOps::mark_ready(canister_id, Cycles::new(50), 30)
            .expect("publish refreshed live balance");
        assert!(CanisterPoolOps::has_ready_asset_for(&Cycles::new(50)));
        assert_eq!(
            CanisterPoolOps::prepare_ready_reinspection(canister_id, &Cycles::new(50), 40)
                .expect("sufficient import replay needs no second reset"),
            CanisterPoolResetPreparation::Ready
        );
        assert_eq!(
            CanisterPoolOps::prepare_import_reinspection(canister_id, &Cycles::new(50), 41)
                .expect("an explicit import refreshes the retained live balance"),
            CanisterPoolResetPreparation::Reinspect
        );
        assert_eq!(
            CanisterPoolOps::pending_reset_canisters(),
            vec![canister_id]
        );
        CanisterPoolOps::mark_ready(canister_id, Cycles::new(55), 42)
            .expect("publish explicitly refreshed balance");
        assert!(CanisterPoolOps::has_ready_asset_for(&Cycles::new(55)));
        assert!(CanisterPoolOps::retry_reset(canister_id, &Cycles::new(50), 40).is_err());
        CanisterPoolStore::clear();
    }

    #[test]
    fn recycled_assets_remain_visible_above_the_import_ceiling() {
        CanisterPoolStore::clear();
        for byte in 1..=4 {
            imported_ready(principal(byte), Cycles::new(100), u64::from(byte));
        }

        let claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([5; 32]),
            operation_id: [5; 32],
        };
        let recycled =
            CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(100), 5)
                .expect("claim")
                .expect("ready asset");
        CanisterPoolOps::finalize_claim(&claim, recycled, 5).expect("workload");
        imported_ready(principal(5), Cycles::new(100), 6);
        CanisterPoolOps::register_recycled_pending(recycled, 6)
            .expect("recycled asset remains managed");
        CanisterPoolOps::register_recycled_pending(recycled, 7)
            .expect("exact recycle retry remains idempotent");
        assert_eq!(CanisterPoolOps::workload_count(), 1);
        assert_eq!(CanisterPoolOps::pooled_asset_count(), 4);
        CanisterPoolOps::validate_complete_recycling(recycled, claim.component)
            .expect_err("Registry membership cannot settle before reset is terminal");
        CanisterPoolOps::mark_ready(recycled, Cycles::new(100), 8)
            .expect("record terminal physical reset");
        CanisterPoolOps::validate_complete_recycling(recycled, claim.component)
            .expect("terminal recycling is safe to settle with membership");
        assert_eq!(CanisterPoolOps::workload_count(), 1);
        assert_eq!(CanisterPoolOps::pooled_asset_count(), 4);
        CanisterPoolOps::complete_recycling(recycled, claim.component, 9)
            .expect("settle Registry membership into pool state");
        CanisterPoolOps::complete_recycling(recycled, claim.component, 10)
            .expect("exact recycling settlement replay");
        CanisterPoolOps::validate_complete_recycling(recycled, claim.component)
            .expect("terminal recycling settlement remains exact-retry safe");
        let response = CanisterPoolOps::response(config(), None, 2);
        assert_eq!(response.tracked, 5);
        assert_eq!(response.pooled, 5);
        assert_eq!(response.surplus, 1);
        assert_eq!(response.recycling, 0);
        assert_eq!(response.entries.len(), 2);
        let cursor = response.next_start_after.expect("additional pool page");
        let final_page = CanisterPoolOps::response(config(), Some(cursor), 3);
        assert_eq!(final_page.tracked, 5);
        assert_eq!(final_page.entries.len(), 3);
        assert_eq!(final_page.next_start_after, None);
        CanisterPoolStore::clear();
    }

    #[test]
    fn draining_handoff_is_exclusive_replayable_and_removes_the_asset() {
        CanisterPoolStore::clear();
        let canister_id = principal(1);
        let recipient = principal(9);
        imported_ready(canister_id, Cycles::new(100), 10);

        let handoff =
            CanisterPoolOps::begin_handoff(canister_id, recipient, 20).expect("begin handoff");
        assert_eq!(
            CanisterPoolOps::begin_handoff(canister_id, recipient, 30)
                .expect("exact handoff replay"),
            handoff
        );
        let claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([7; 32]),
            operation_id: [7; 32],
        };
        assert_eq!(
            CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(100), 40)
                .expect("handoff asset is not claimable"),
            None
        );
        let response = CanisterPoolOps::response(config(), None, 10);
        assert_eq!(response.handing_off, 1);
        assert_eq!(
            response.pending_handoff,
            Some(CanisterPoolHandoff {
                canister_id,
                recipient,
                prepared_at_ns: 20,
            })
        );

        CanisterPoolOps::complete_handoff(canister_id, recipient, 50).expect("complete handoff");
        assert_eq!(CanisterPoolOps::pooled_asset_count(), 0);
        assert_eq!(CanisterPoolOps::pending_handoff(), None);
        assert_eq!(
            CanisterPoolOps::completed_handoff_recipient(canister_id),
            Some(recipient)
        );
        assert_eq!(
            CanisterPoolOps::response(config(), None, 10).completed_handoffs,
            1
        );
        CanisterPoolStore::clear();
    }
}
