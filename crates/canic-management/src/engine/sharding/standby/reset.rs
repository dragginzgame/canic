//! Only the exact released worker may be stopped and uninstalled.
use super::{Asset, MANAGEMENT_LIMITS, STATE, api, management, mgmt};
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
) -> Result<ResetPhase, String> {
    assert!(api::is_controller(&api::caller()));
    let slot = ChildId::placement(target.allocation.slot as usize).ok_or("invalid worker slot")?;
    let _lock = super::super::super::observing(slot).ok_or("worker observation in progress")?;
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
            Err("reset requires the original deadline".into())
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
            .ok_or("released worker allocation required")?;
        if matches!(asset, Asset::Reset(_, _, original, _) if *original != deadline) {
            return Err("reset requires the original deadline");
        }
        let workers = state.pools.workers.as_ref().ok_or("missing worker grant")?;
        let minimum = workers.minimum_reuse_cycles()?;
        let journal = &state.journal.0[&slot];
        if journal.1.approval.is_some() {
            return Err("worker has separate upgrade custody");
        }
        let child = &journal.0;
        if child.phase != Phase::Ready || child.worker.as_deref() != Some(&target.shard.to_text()) {
            return Err("worker creation is unsettled");
        }
        Ok((
            index,
            asset.clone(),
            minimum,
            child
                .observed_module
                .clone()
                .ok_or("missing installed module")?,
        ))
    })?;
    let args = mgmt::CanisterArgs {
        canister_id: target.shard,
    }
    .encode(1024)
    .expect("principal");
    let inventory = management("list_canister_snapshots", args.clone(), 0)
        .await
        .map_err(|e| format!("snapshot inventory: {e:?}"))?;
    if !decode_list_snapshots(&inventory, MANAGEMENT_LIMITS)
        .map_err(|_| "invalid snapshot inventory")?
        .is_empty()
    {
        return Err("worker has retained snapshots".into());
    }
    let reply = management("canister_status", args.clone(), 0)
        .await
        .map_err(|e| format!("worker status: {e:?}"))?;
    let status = mgmt::decode_canister_status(&reply, MANAGEMENT_LIMITS)
        .map_err(|_| "invalid worker status")?;
    if status.settings.controllers != [api::canister_self()] {
        return Err("Root must be sole controller".into());
    }
    let balance = if matches!(
        before,
        Asset::Reset(_, _, _, ResetPhase::Uninstalled | ResetPhase::StartIssued)
    ) {
        let resources = mgmt::decode_canister_resources(&reply, MANAGEMENT_LIMITS)
            .map_err(|_| "missing worker resource evidence")?;
        if resources.memory_metrics.snapshots_size != 0_u64.into() {
            return Err("worker snapshot memory is occupied".into());
        }
        let balance = u128::try_from(resources.cycles.0)
            .map_err(|_| "worker balance exceeds accounting range")?;
        if balance < u128::from(minimum.get()) {
            return Err(format!(
                "worker has {balance} cycles; return requires at least {minimum}"
            ));
        }
        Some(balance)
    } else {
        None
    };
    let (phase, method) = match (&before, status.status, status.module_hash) {
        (Asset::Reset(_, _, _, ResetPhase::StartIssued), Status::Running, None) => {
            (ResetPhase::Returned, None)
        }
        (Asset::Reset(_, _, _, ResetPhase::Uninstalled), Status::Stopped, None) => {
            (ResetPhase::StartIssued, Some("start_canister"))
        }
        (Asset::Reset(_, _, _, ResetPhase::UninstallIssued), Status::Stopped, None) => {
            (ResetPhase::Uninstalled, None)
        }
        (
            Asset::Released(..) | Asset::Reset(_, _, _, ResetPhase::StopIssued),
            Status::Stopped,
            Some(actual),
        ) if actual == hash => (ResetPhase::UninstallIssued, Some("uninstall_code")),
        (Asset::Released(..), Status::Running, Some(actual)) if actual == hash => {
            (ResetPhase::StopIssued, Some("stop_canister"))
        }
        _ => return Err("worker status does not yet confirm the expected reset phase".into()),
    };
    admit_receiver(target.shard)?;
    super::super::admit_pool_change(false).map_err(|e| format!("outstanding custody: {e:?}"))?;
    let now = api::time();
    if method.is_some() && (deadline <= now || deadline - now > 600_000_000_000) {
        return Err("reset dispatch requires its original deadline within ten minutes".into());
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let asset = &mut state.pools.standby[index].1;
        if asset != &before {
            return Err("worker reset claim changed");
        }
        if phase == ResetPhase::Returned {
            let balance = balance.expect("verified return balance");
            *asset = Asset::Available(target.shard, balance);
            state.pools.returns.push(super::Returned {
                target,
                deadline,
                balance,
            });
        } else {
            *asset = Asset::Reset(target.shard, target.allocation, deadline, phase);
        }
        state.pool.set(state.pools.encode());
        Ok(())
    })?;
    if let Some(method) = method {
        management(method, args, 0)
            .await
            .map_err(|e| format!("{method} issued; observe original reset: {e:?}"))?;
    }
    Ok(phase)
}
