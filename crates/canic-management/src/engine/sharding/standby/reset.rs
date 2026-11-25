//! Only the exact released worker may be stopped and uninstalled.
use super::{Asset, MANAGEMENT_LIMITS, STATE, api, management, mgmt};
use canic::sharding::standby::ReserveError;
use canic::sharding::{ShardTarget, standby::ResetPhase};
use canic_management_protocol::{Phase, children::ChildId};
use ic_platform_protocol::management_snapshots::{Status, decode_list_snapshots};

/// Advance or reconcile the controller's exact worker reset approval.
/// # Errors
/// Refuses outstanding custody, changed bindings, expired dispatch or missing evidence.
/// # Panics
/// Requires an opened Root and its actual controller.
pub async fn reset(
    target: ShardTarget,
    deadline: u64,
    admit_receiver: impl FnOnce(ic_platform_types::Principal) -> Result<(), String>,
) -> Result<ResetPhase, ReserveError> {
    let controller = api::caller();
    assert!(api::is_controller(&controller));
    let slot = ChildId::placement(target.allocation.slot as usize)
        .ok_or_else(|| ReserveError::InvalidApproval("invalid worker slot".into()))?;
    if !ChildId::INSTANCES.contains(&slot) {
        return Err(ReserveError::WrongAsset(
            "released worker allocation required".into(),
        ));
    }
    let _lock = super::super::super::observing(slot)
        .ok_or_else(|| ReserveError::CustodyOccupied("worker observation in progress".into()))?;
    if let Some(original) = STATE.with_borrow(|s| {
        s.as_ref()
            .unwrap()
            .pools
            .returns
            .iter()
            .find(|r| r.target == target)
            .map(|r| r.deadline)
    }) {
        return if original == deadline {
            Ok(ResetPhase::Returned)
        } else {
            Err(ReserveError::ApprovalConflict(
                "reset requires the original deadline".into(),
            ))
        };
    }
    let (index, before, minimum, hash) = STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let (index, (_, asset)) = state
            .pools
            .standby
            .iter()
            .enumerate()
            .find(|(_, (_, asset))| {
                super::released_binding(asset) == Some((target.shard, &target.allocation))
            })
            .ok_or_else(|| {
                ReserveError::WrongAsset("released worker allocation required".into())
            })?;
        if matches!(asset, Asset::Reset(_, _, original, _) if *original != deadline) {
            return Err(ReserveError::ApprovalConflict(
                "reset requires the original deadline".into(),
            ));
        }
        let (workers, _) = state
            .pools
            .worker_at(slot)
            .ok_or_else(|| ReserveError::NotConfigured("missing worker grant".into()))?;
        let minimum = workers
            .minimum_reuse_cycles()
            .map_err(|e| ReserveError::NotConfigured(e.into()))?;
        let journal = &state.journal.0[&slot];
        if journal.1.approval.is_some() {
            return Err(ReserveError::CustodyOccupied(
                "worker has separate upgrade custody".into(),
            ));
        }
        let child = &journal.0;
        if child.phase != Phase::Ready || child.worker.as_deref() != Some(&target.shard.to_text()) {
            return Err(ReserveError::CustodyOccupied(
                "worker creation is unsettled".into(),
            ));
        }
        Ok((
            index,
            asset.clone(),
            minimum,
            child
                .observed_module
                .clone()
                .ok_or_else(|| ReserveError::WrongAsset("missing installed module".into()))?,
        ))
    })?;
    let args = mgmt::CanisterArgs {
        canister_id: target.shard,
    }
    .encode(1024)
    .expect("principal");
    let prior_phase = match before {
        Asset::Reset(_, _, _, phase) => Some(phase),
        _ => None,
    };
    let (phase, method, balance) = observe_reset(&args, prior_phase, &hash, minimum).await?;
    admit_receiver(target.shard).map_err(ReserveError::CustodyOccupied)?;
    super::authorize_change(controller)?;
    let now = api::time();
    if method.is_some() && (deadline <= now || deadline - now > 600_000_000_000) {
        return Err(ReserveError::Deadline(
            "reset dispatch requires its original deadline within ten minutes".into(),
        ));
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let asset = &mut state.pools.standby[index].1;
        if asset != &before {
            return Err(ReserveError::ApprovalConflict(
                "worker reset claim changed".into(),
            ));
        }
        if phase == ResetPhase::Returned {
            let balance = balance.expect("verified return balance");
            *asset = Asset::Available(target.shard, balance);
            super::super::workers::archive(state, slot, target.allocation.generation);
            state.pools.returns.push(super::Returned {
                target,
                deadline,
                balance,
            });
            super::log_surplus(&state.pools);
        } else {
            *asset = Asset::Reset(target.shard, target.allocation, deadline, phase);
        }
        state.pool.set(state.pools.encode());
        Ok(())
    })?;
    if let Some(method) = method {
        management(method, args, 0).await.map_err(|e| {
            ReserveError::EffectIssued(format!("{method} issued; observe original reset: {e:?}"))
        })?;
        #[cfg(feature = "fault-injection")]
        if phase == ResetPhase::UninstallIssued {
            // Root-wide uninstall callback fault, selected on the same channel as reply faults.
            super::super::super::fault(ChildId::FIRST, 26);
        }
    }
    Ok(phase)
}

pub(super) async fn observe_reset(
    args: &[u8],
    before: Option<ResetPhase>,
    hash: &[u8],
    minimum: std::num::NonZeroU64,
) -> Result<(ResetPhase, Option<&'static str>, Option<u128>), ReserveError> {
    let (status, balance) = observe_target(args).await?;
    let balance = if matches!(
        before,
        Some(ResetPhase::Uninstalled | ResetPhase::StartIssued)
    ) {
        if balance < u128::from(minimum.get()) {
            return Err(ReserveError::BalanceBelowMinimum(format!(
                "worker has {balance} cycles; return requires at least {minimum}"
            )));
        }
        Some(balance)
    } else {
        None
    };
    let (phase, method) = match (before, status.status, status.module_hash) {
        (Some(ResetPhase::StartIssued), Status::Running, None) => (ResetPhase::Returned, None),
        (Some(ResetPhase::Uninstalled), Status::Stopped, None) => {
            (ResetPhase::StartIssued, Some("start_canister"))
        }
        (Some(ResetPhase::UninstallIssued), Status::Stopped, None) => {
            (ResetPhase::Uninstalled, None)
        }
        (None | Some(ResetPhase::StopIssued), Status::Stopped, Some(actual)) if actual == hash => {
            (ResetPhase::UninstallIssued, Some("uninstall_code"))
        }
        (None, Status::Running, Some(actual)) if actual == hash => {
            (ResetPhase::StopIssued, Some("stop_canister"))
        }
        _ => {
            return Err(ReserveError::AwaitingConfirmation(
                "worker status does not yet confirm the expected reset phase".into(),
            ));
        }
    };
    Ok((phase, method, balance))
}

/// Observe snapshot custody, sole control, module and balance from management.
pub(in crate::engine::sharding) async fn observe_target(
    args: &[u8],
) -> Result<(mgmt::CanisterStatus, u128), ReserveError> {
    let inventory = management("list_canister_snapshots", args.to_vec(), 0)
        .await
        .map_err(|e| ReserveError::AwaitingConfirmation(format!("snapshot inventory: {e:?}")))?;
    if !decode_list_snapshots(&inventory, MANAGEMENT_LIMITS)
        .map_err(|_| ReserveError::InvalidObservation("invalid snapshot inventory".into()))?
        .is_empty()
    {
        return Err(ReserveError::CustodyOccupied(
            "target has retained snapshots".into(),
        ));
    }
    let reply = management("canister_status", args.to_vec(), 0)
        .await
        .map_err(|e| ReserveError::AwaitingConfirmation(format!("target status: {e:?}")))?;
    let status = mgmt::decode_canister_status(&reply, MANAGEMENT_LIMITS)
        .map_err(|_| ReserveError::InvalidObservation("invalid target status".into()))?;
    if status.settings.controllers != [api::canister_self()] {
        return Err(ReserveError::Unauthorized(
            "Root must be sole controller".into(),
        ));
    }
    let resources = mgmt::decode_canister_resources(&reply, MANAGEMENT_LIMITS)
        .map_err(|_| ReserveError::InvalidObservation("missing target resource evidence".into()))?;
    if resources.memory_metrics.snapshots_size != 0_u64.into() {
        return Err(ReserveError::CustodyOccupied(
            "target snapshot memory is occupied".into(),
        ));
    }
    let balance = u128::try_from(resources.cycles.0).map_err(|_| {
        ReserveError::InvalidObservation("target balance exceeds accounting range".into())
    })?;
    Ok((status, balance))
}
