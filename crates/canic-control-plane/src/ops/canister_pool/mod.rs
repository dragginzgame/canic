//! Deterministic state access and DTO conversion for root-owned physical Canisters.

use crate::storage::stable::canister_pool::{
    CanisterPoolAssetOriginRecord, CanisterPoolAssetRecord, CanisterPoolAssetStatusRecord,
    CanisterPoolClaimRecord, CanisterPoolHandoffReceiptRecord, CanisterPoolHandoffRecord,
    CanisterPoolRecycleResetRecord, CanisterPoolStore,
};
use crate::view::canister_pool::CanisterPoolHandoffView;
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::error::{InternalError, InternalErrorOrigin},
    dto::pool::{
        CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus, CanisterPoolClaim,
        CanisterPoolHandoff, CanisterPoolRecycleReset, CanisterPoolResponse,
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
            Some(_) => Err(InternalError::conflict(
                "sibling Wasm Store principal conflicts with root physical Canister inventory",
            )),
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
            return Err(InternalError::invalid_input(
                "Canister pool imports exceed the configured maximum_size",
            ));
        }
        let unique = imports.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != imports.len() {
            return Err(InternalError::invalid_input(
                "Canister pool imports contain duplicate principals",
            ));
        }

        for canister_id in imports {
            match CanisterPoolStore::get(canister_id) {
                Some(existing) if existing.origin == CanisterPoolAssetOriginRecord::Imported => {}
                Some(_) => {
                    return Err(InternalError::conflict(format!(
                        "Canister pool import {canister_id} conflicts with an existing asset"
                    )));
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
            _ => Err(InternalError::conflict(format!(
                "recycled Canister {canister_id} is not an exact workload asset"
            ))),
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
                return Err(InternalError::conflict(
                    "only pending, recycling or failed Canister pool assets may complete reset",
                ));
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
                return Err(InternalError::conflict(
                    "only a pending, recycling or failed Canister pool asset may enter reset failure",
                ));
            }
        };
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
    }

    pub fn retry_reset(canister_id: Principal, now_ns: u64) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        asset.status = match asset.status {
            CanisterPoolAssetStatusRecord::Failed(_) => CanisterPoolAssetStatusRecord::PendingReset,
            CanisterPoolAssetStatusRecord::Recycling {
                claim,
                reset: CanisterPoolRecycleResetRecord::Failed(_),
            } => CanisterPoolAssetStatusRecord::Recycling {
                claim,
                reset: CanisterPoolRecycleResetRecord::Pending,
            },
            _ => {
                return Err(InternalError::conflict(
                    "only a failed Canister pool asset may retry reset",
                ));
            }
        };
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
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

    pub fn claim_oldest_ready(
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
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "one Component allocation claims multiple Canister pool assets",
            ));
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
                    .added_at_ns
                    .cmp(&right.asset.added_at_ns)
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
            _ => Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "one Component allocation claims multiple Canister pool assets",
            )),
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
            _ => Err(InternalError::conflict(
                "Canister pool claim differs from the Component allocation",
            )),
        }
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
            handing_off,
            failed,
            completed_handoffs: CanisterPoolStore::handoff_receipt_count(),
            pending_handoff: state.handoff.map(|handoff| CanisterPoolHandoff {
                canister_id: handoff.canister_id,
                recipient: handoff.recipient,
                prepared_at_ns: handoff.prepared_at_ns,
            }),
            entries: page,
            next_start_after,
        }
    }

    pub fn begin_handoff(
        canister_id: Principal,
        recipient: Principal,
        prepared_at_ns: u64,
    ) -> Result<CanisterPoolHandoffView, InternalError> {
        if CanisterPoolStore::handoff_receipt(&canister_id).is_some() {
            return Err(InternalError::conflict(
                "Canister pool asset handoff is already complete",
            ));
        }
        let mut state = CanisterPoolStore::state();
        if let Some(existing) = state.handoff {
            if existing.canister_id == canister_id && existing.recipient == recipient {
                let asset = required_asset(canister_id)?;
                if asset.status == (CanisterPoolAssetStatusRecord::HandingOff { recipient }) {
                    return Ok(CanisterPoolHandoffView {
                        canister_id,
                        recipient,
                    });
                }
                return Err(InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "Canister pool handoff journal differs from asset state",
                ));
            }
            return Err(InternalError::conflict(
                "another Canister pool handoff is already pending",
            ));
        }
        let mut asset = required_asset(canister_id)?;
        if !matches!(
            asset.status,
            CanisterPoolAssetStatusRecord::Ready | CanisterPoolAssetStatusRecord::Failed(_)
        ) {
            return Err(InternalError::conflict(
                "only a ready or failed Canister pool asset may be handed off",
            ));
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
        let handoff = state
            .handoff
            .ok_or_else(|| InternalError::unavailable("Canister pool handoff is not pending"))?;
        if handoff.canister_id != canister_id || handoff.recipient != recipient {
            return Err(InternalError::conflict(
                "Canister pool handoff completion differs from pending authority",
            ));
        }
        let asset = required_asset(canister_id)?;
        if asset.status != (CanisterPoolAssetStatusRecord::HandingOff { recipient }) {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Canister pool handoff asset differs from pending authority",
            ));
        }
        if CanisterPoolStore::handoff_receipt(&canister_id).is_some() {
            return Err(InternalError::conflict(
                "Canister pool handoff receipt already exists",
            ));
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

    #[must_use]
    pub fn completed_handoff_recipient(canister_id: Principal) -> Option<Principal> {
        CanisterPoolStore::handoff_receipt(&canister_id).map(|receipt| receipt.recipient)
    }

    pub fn asset_is_ready(canister_id: Principal) -> Result<bool, InternalError> {
        Ok(matches!(
            required_asset(canister_id)?.status,
            CanisterPoolAssetStatusRecord::Ready
        ))
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
            return Err(InternalError::conflict(
                "sibling Wasm Store differs from root physical Canister inventory",
            ));
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
            return Err(InternalError::conflict(
                "Store deletion target is not root infrastructure inventory",
            ));
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
            _ => Err(InternalError::conflict(
                "Store deletion differs from root physical Canister inventory",
            )),
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
            return Err(InternalError::conflict(
                "completed Store deletion differs from root physical Canister inventory",
            ));
        }
        CanisterPoolStore::remove(&canister_id);
        Ok(())
    }
}

fn validate_config(config: &FleetSubnetCanisterPoolConfig) -> Result<(), InternalError> {
    if config.minimum_size == 0
        || config.maximum_size < config.minimum_size
        || config.canister_cycles.to_u128() == 0
    {
        return Err(InternalError::invalid_input(
            "Fleet Subnet Root Canister pool policy is invalid",
        ));
    }
    Ok(())
}

fn validate_new_asset_capacity(
    config: &FleetSubnetCanisterPoolConfig,
    canister_id: Principal,
) -> Result<(), InternalError> {
    validate_config(config)?;
    if CanisterPoolStore::get(&canister_id).is_none()
        && CanisterPoolOps::pooled_asset_count() >= config.maximum_size
    {
        return Err(InternalError::resource_exhausted(
            "Canister pool maximum_size is exhausted",
        ));
    }
    Ok(())
}

fn required_asset(canister_id: Principal) -> Result<CanisterPoolAssetRecord, InternalError> {
    CanisterPoolStore::get(&canister_id).ok_or_else(|| {
        InternalError::unavailable(format!(
            "Canister pool asset {canister_id} is not registered"
        ))
    })
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
        return Err(InternalError::conflict(
            "Component membership removal differs from physical recycling inventory",
        ));
    };
    if claim.component != component {
        return Err(InternalError::conflict(
            "Component membership removal names a different physical workload owner",
        ));
    }
    let next_status = match reset {
        CanisterPoolRecycleResetRecord::Ready => CanisterPoolAssetStatusRecord::Ready,
        CanisterPoolRecycleResetRecord::Failed(reason) => {
            CanisterPoolAssetStatusRecord::Failed(reason.clone())
        }
        CanisterPoolRecycleResetRecord::Pending => {
            return Err(InternalError::unavailable(
                "Component workload recycling reset has not reached a terminal outcome",
            ));
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

fn count_as_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn asset_to_dto(canister_id: Principal, asset: CanisterPoolAssetRecord) -> CanisterPoolAsset {
    CanisterPoolAsset {
        canister_id,
        cycles: asset.cycles,
        origin: match asset.origin {
            CanisterPoolAssetOriginRecord::InfrastructureStore => {
                CanisterPoolAssetOrigin::InfrastructureStore
            }
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

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
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

    #[test]
    fn claim_is_oldest_first_and_exactly_replayable() {
        CanisterPoolStore::clear();
        imported_ready(principal(2), Cycles::new(100), 20);
        imported_ready(principal(1), Cycles::new(100), 10);
        let claim = CanisterPoolClaimKey {
            component: ComponentInstanceId::from_generated_bytes([7; 32]),
            operation_id: [7; 32],
        };

        let selected = CanisterPoolOps::claim_oldest_ready(&claim, &Cycles::new(100), 30)
            .expect("claim")
            .expect("ready asset");
        assert_eq!(selected, principal(1));
        assert_eq!(
            CanisterPoolOps::claim_oldest_ready(&claim, &Cycles::new(100), 40)
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
            CanisterPoolOps::claim_oldest_ready(&claim, &Cycles::new(100), 3)
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
            CanisterPoolOps::claim_oldest_ready(&claim, &Cycles::new(100), 20)
                .expect("claim decision"),
            None
        );
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
        let recycled = CanisterPoolOps::claim_oldest_ready(&claim, &Cycles::new(100), 5)
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
            CanisterPoolOps::claim_oldest_ready(&claim, &Cycles::new(100), 40)
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
