//! Deterministic state access and DTO conversion for the root-owned Canister pool.

use crate::storage::stable::canister_pool::{
    CanisterPoolAssetOriginRecord, CanisterPoolAssetRecord, CanisterPoolAssetStatusRecord,
    CanisterPoolClaimRecord, CanisterPoolCreationProgressRecord, CanisterPoolCreationRecord,
    CanisterPoolHandoffReceiptRecord, CanisterPoolHandoffRecord, CanisterPoolStore,
};
use crate::view::canister_pool::{CanisterPoolCreationView, CanisterPoolHandoffView};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        model::replay::ReplayCostGuardSettlement,
    },
    dto::pool::{
        CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus, CanisterPoolClaim,
        CanisterPoolCreation, CanisterPoolHandoff, CanisterPoolResponse,
    },
    ids::{ComponentInstanceId, FleetSubnetCanisterPoolConfig},
};
use std::collections::BTreeSet;

/// Stable identity of one Component allocation claiming a prepaid asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterPoolClaimKey {
    pub component: Option<ComponentInstanceId>,
    pub operation_id: [u8; 32],
}

/// Mechanical state facade for the Fleet Subnet Root's prepaid assets.
pub struct CanisterPoolOps;

impl CanisterPoolOps {
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
                    CanisterPoolStore::insert(
                        *canister_id,
                        CanisterPoolAssetRecord {
                            cycles: Cycles::default(),
                            origin: CanisterPoolAssetOriginRecord::Imported,
                            status: CanisterPoolAssetStatusRecord::PendingReset,
                            added_at_ns: now_ns,
                            updated_at_ns: now_ns,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    pub fn register_created_ready(
        config: &FleetSubnetCanisterPoolConfig,
        canister_id: Principal,
        cycles: Cycles,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        Self::register_ready(
            config,
            canister_id,
            cycles,
            CanisterPoolAssetOriginRecord::Created,
            now_ns,
        )
    }

    pub fn register_recycled_pending(
        canister_id: Principal,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        if let Some(existing) = CanisterPoolStore::get(&canister_id) {
            if existing.origin == CanisterPoolAssetOriginRecord::Recycled
                && matches!(
                    existing.status,
                    CanisterPoolAssetStatusRecord::PendingReset
                        | CanisterPoolAssetStatusRecord::Ready
                        | CanisterPoolAssetStatusRecord::Failed(_)
                )
            {
                return Ok(());
            }
            return Err(InternalError::conflict(format!(
                "recycled Canister pool asset {canister_id} conflicts with existing inventory"
            )));
        }
        CanisterPoolStore::insert(
            canister_id,
            CanisterPoolAssetRecord {
                cycles: Cycles::default(),
                origin: CanisterPoolAssetOriginRecord::Recycled,
                status: CanisterPoolAssetStatusRecord::PendingReset,
                added_at_ns: now_ns,
                updated_at_ns: now_ns,
            },
        );
        Ok(())
    }

    pub fn mark_ready(
        canister_id: Principal,
        cycles: Cycles,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        if !matches!(
            asset.status,
            CanisterPoolAssetStatusRecord::PendingReset | CanisterPoolAssetStatusRecord::Failed(_)
        ) {
            return Err(InternalError::conflict(
                "only pending or failed Canister pool assets may become ready",
            ));
        }
        asset.cycles = cycles;
        asset.status = CanisterPoolAssetStatusRecord::Ready;
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
        if matches!(
            asset.status,
            CanisterPoolAssetStatusRecord::Claimed(_)
                | CanisterPoolAssetStatusRecord::HandingOff { .. }
        ) {
            return Err(InternalError::conflict(
                "a claimed or handing-off Canister pool asset cannot enter reset failure",
            ));
        }
        if let Some(cycles) = observed_cycles {
            asset.cycles = cycles;
        }
        asset.status = CanisterPoolAssetStatusRecord::Failed(reason);
        asset.updated_at_ns = now_ns;
        CanisterPoolStore::insert(canister_id, asset);
        Ok(())
    }

    pub fn retry_reset(canister_id: Principal, now_ns: u64) -> Result<(), InternalError> {
        let mut asset = required_asset(canister_id)?;
        if !matches!(asset.status, CanisterPoolAssetStatusRecord::Failed(_)) {
            return Err(InternalError::conflict(
                "only a failed Canister pool asset may retry reset",
            ));
        }
        asset.status = CanisterPoolAssetStatusRecord::PendingReset;
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
    ) -> Result<(), InternalError> {
        let asset = required_asset(canister_id)?;
        if asset.status != CanisterPoolAssetStatusRecord::Claimed(claim_record(claim)) {
            return Err(InternalError::conflict(
                "Canister pool claim differs from the Component allocation",
            ));
        }
        CanisterPoolStore::remove(&canister_id);
        Ok(())
    }

    pub fn response(
        config: FleetSubnetCanisterPoolConfig,
        start_after: Option<Principal>,
        limit: usize,
    ) -> CanisterPoolResponse {
        let data = CanisterPoolStore::export();
        let state = data.state;
        let mut ready = 0_u32;
        let mut pending_reset = 0_u32;
        let mut claimed = 0_u32;
        let mut handing_off = 0_u32;
        let mut failed = 0_u32;
        let all_entries: Vec<CanisterPoolAsset> = data
            .entries
            .into_iter()
            .map(|entry| {
                match &entry.asset.status {
                    CanisterPoolAssetStatusRecord::PendingReset => pending_reset += 1,
                    CanisterPoolAssetStatusRecord::Ready => ready += 1,
                    CanisterPoolAssetStatusRecord::Claimed(_) => claimed += 1,
                    CanisterPoolAssetStatusRecord::HandingOff { .. } => handing_off += 1,
                    CanisterPoolAssetStatusRecord::Failed(_) => failed += 1,
                }
                asset_to_dto(entry.canister_id, entry.asset)
            })
            .collect();
        let tracked = count_as_u32(all_entries.len());
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
            surplus: tracked.saturating_sub(config.maximum_size),
            tracked,
            config,
            ready,
            pending_reset,
            claimed,
            handing_off,
            failed,
            completed_handoffs: CanisterPoolStore::handoff_receipt_count(),
            pending_creation: state.creation.map(|creation| CanisterPoolCreation {
                operation_id: creation.operation_id,
                canister_cycles: creation.canister_cycles,
                canister_id: match creation.progress {
                    CanisterPoolCreationProgressRecord::Intent => None,
                    CanisterPoolCreationProgressRecord::Created { canister_id } => {
                        Some(canister_id)
                    }
                },
                prepared_at_ns: creation.prepared_at_ns,
            }),
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
        operation_id: [u8; 32],
        canister_cycles: Cycles,
        cost_guard_settlement: ReplayCostGuardSettlement,
        prepared_at_ns: u64,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        if let Some(existing) = state.creation {
            if existing.operation_id == operation_id
                && existing.canister_cycles == canister_cycles
                && existing.cost_guard_settlement == cost_guard_settlement
            {
                return Ok(());
            }
            return Err(InternalError::conflict(
                "another Canister pool creation is already pending",
            ));
        }
        let creation = CanisterPoolCreationRecord {
            operation_id,
            canister_cycles,
            cost_guard_settlement,
            prepared_at_ns,
            progress: CanisterPoolCreationProgressRecord::Intent,
        };
        state.creation = Some(creation);
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn mark_creation_created(
        operation_id: [u8; 32],
        canister_id: Principal,
    ) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let mut creation = state.creation.ok_or_else(|| {
            InternalError::unavailable("Canister pool creation intent is not pending")
        })?;
        if creation.operation_id != operation_id {
            return Err(InternalError::conflict(
                "Canister pool creation operation identity differs",
            ));
        }
        match creation.progress {
            CanisterPoolCreationProgressRecord::Intent => {
                creation.progress = CanisterPoolCreationProgressRecord::Created { canister_id };
            }
            CanisterPoolCreationProgressRecord::Created {
                canister_id: existing,
            } if existing == canister_id => return Ok(()),
            CanisterPoolCreationProgressRecord::Created { .. } => {
                return Err(InternalError::conflict(
                    "Canister pool creation already recorded another principal",
                ));
            }
        }
        state.creation = Some(creation);
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    pub fn commit_creation(operation_id: [u8; 32]) -> Result<(), InternalError> {
        let mut state = CanisterPoolStore::state();
        let creation = state
            .creation
            .as_ref()
            .ok_or_else(|| InternalError::unavailable("Canister pool creation is not pending"))?;
        if creation.operation_id != operation_id
            || !matches!(
                creation.progress,
                CanisterPoolCreationProgressRecord::Created { .. }
            )
        {
            return Err(InternalError::conflict(
                "Canister pool creation cannot commit from its current authority",
            ));
        }
        state.next_creation_sequence =
            state.next_creation_sequence.checked_add(1).ok_or_else(|| {
                InternalError::resource_exhausted("Canister pool creation sequence is exhausted")
            })?;
        state.creation = None;
        CanisterPoolStore::set_state(state);
        Ok(())
    }

    #[must_use]
    pub fn next_creation_sequence() -> u64 {
        CanisterPoolStore::state().next_creation_sequence
    }

    pub fn pending_creation() -> Option<CanisterPoolCreationView> {
        CanisterPoolStore::state()
            .creation
            .map(|creation| CanisterPoolCreationView {
                operation_id: creation.operation_id,
                canister_cycles: creation.canister_cycles,
                cost_guard_settlement: creation.cost_guard_settlement,
                canister_id: match creation.progress {
                    CanisterPoolCreationProgressRecord::Intent => None,
                    CanisterPoolCreationProgressRecord::Created { canister_id } => {
                        Some(canister_id)
                    }
                },
            })
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
        if state.creation.is_some() {
            return Err(InternalError::unavailable(
                "Canister pool creation must reconcile before asset handoff",
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
    pub fn asset_count() -> u32 {
        count_as_u32(CanisterPoolStore::export().entries.len())
    }

    fn register_ready(
        config: &FleetSubnetCanisterPoolConfig,
        canister_id: Principal,
        cycles: Cycles,
        origin: CanisterPoolAssetOriginRecord,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        validate_new_asset_capacity(config, canister_id)?;
        CanisterPoolStore::insert(
            canister_id,
            CanisterPoolAssetRecord {
                cycles,
                origin,
                status: CanisterPoolAssetStatusRecord::Ready,
                added_at_ns: now_ns,
                updated_at_ns: now_ns,
            },
        );
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
        && CanisterPoolOps::asset_count() >= config.maximum_size
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
            CanisterPoolAssetOriginRecord::Created => CanisterPoolAssetOrigin::Created,
            CanisterPoolAssetOriginRecord::Imported => CanisterPoolAssetOrigin::Imported,
            CanisterPoolAssetOriginRecord::Recycled => CanisterPoolAssetOrigin::Recycled,
        },
        status: match asset.status {
            CanisterPoolAssetStatusRecord::PendingReset => CanisterPoolAssetStatus::PendingReset,
            CanisterPoolAssetStatusRecord::Ready => CanisterPoolAssetStatus::Ready,
            CanisterPoolAssetStatusRecord::Claimed(claim) => CanisterPoolAssetStatus::Claimed {
                claim: CanisterPoolClaim {
                    component: claim.component,
                    operation_id: claim.operation_id,
                },
            },
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

    #[test]
    fn claim_is_oldest_first_and_exactly_replayable() {
        CanisterPoolStore::clear();
        CanisterPoolOps::register_created_ready(&config(), principal(2), Cycles::new(100), 20)
            .expect("newer asset");
        CanisterPoolOps::register_created_ready(&config(), principal(1), Cycles::new(100), 10)
            .expect("older asset");
        let claim = CanisterPoolClaimKey {
            component: None,
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

        CanisterPoolOps::finalize_claim(&claim, selected).expect("finalize claim");
        assert_eq!(CanisterPoolOps::asset_count(), 1);
        CanisterPoolStore::clear();
    }

    #[test]
    fn insufficient_cycle_assets_are_not_claimed() {
        CanisterPoolStore::clear();
        CanisterPoolOps::register_created_ready(&config(), principal(1), Cycles::new(99), 10)
            .expect("asset");
        let claim = CanisterPoolClaimKey {
            component: None,
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
    fn recycled_assets_remain_visible_above_the_proactive_refill_ceiling() {
        CanisterPoolStore::clear();
        for byte in 1..=4 {
            CanisterPoolOps::register_created_ready(
                &config(),
                principal(byte),
                Cycles::new(100),
                u64::from(byte),
            )
            .expect("configured pool asset");
        }

        CanisterPoolOps::register_recycled_pending(principal(5), 5)
            .expect("recycled asset remains managed");
        CanisterPoolOps::register_recycled_pending(principal(5), 6)
            .expect("exact recycle retry remains idempotent");
        let response = CanisterPoolOps::response(config(), None, 2);
        assert_eq!(response.tracked, 5);
        assert_eq!(response.surplus, 1);
        assert_eq!(response.pending_reset, 1);
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
        CanisterPoolOps::register_created_ready(&config(), canister_id, Cycles::new(100), 10)
            .expect("ready asset");

        let handoff =
            CanisterPoolOps::begin_handoff(canister_id, recipient, 20).expect("begin handoff");
        assert_eq!(
            CanisterPoolOps::begin_handoff(canister_id, recipient, 30)
                .expect("exact handoff replay"),
            handoff
        );
        let claim = CanisterPoolClaimKey {
            component: None,
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
        assert_eq!(CanisterPoolOps::asset_count(), 0);
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
