//! One source approval pins an empty spare to the Root that reserved it.
use super::{Asset, STATE, api, available, management, mgmt, policy, reset};
use canic::sharding::standby::{HandoffStatus, ImportStandby, TransferPhase};
use ic_platform_types::Principal;

// No await separates replacement from persistence. A competing continuation can only observe.
fn replace(index: usize, expected: &Asset, next: Asset) -> Result<(), String> {
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().expect("opened Root");
        if s.pools.reserved_for_payment(index) || s.pools.standby[index].1 != *expected {
            return Err("source inventory advanced; observe the original transfer".into());
        }
        s.pools.standby[index].1 = next;
        s.pool.set(s.pools.encode());
        Ok(())
    })
}

/// Hand off sole control once, then settle only from the named destination's retained custody.
/// # Errors
/// Requires a controller, live dispatch authority, an empty spare and a matching import reservation.
/// # Panics
/// Requires opened Root custody.
pub async fn export(
    id: Principal,
    destination: Principal,
    deadline: u64,
) -> Result<TransferPhase, String> {
    use TransferPhase::{Confirmed, ControllersIssued, Reserved};
    let caller = api::caller();
    super::authorize_change(caller)?;
    if destination.as_slice().last() != Some(&1)
        || destination == id
        || destination == api::canister_self()
    {
        return Err("a distinct destination Root is required".into());
    }
    let (index, before) = STATE
        .with_borrow(|s| {
            s.as_ref()
                .expect("opened Root")
                .pools
                .standby
                .iter()
                .enumerate()
                .filter(|(i, _)| !s.as_ref().unwrap().pools.reserved_for_payment(*i))
                .find_map(|(i, (_, a))| {
                    (available(a).is_some_and(|(subject, _)| subject == id)
                        || matches!(a, Asset::Transfer(subject,..) if *subject == id))
                    .then(|| (i, a.clone()))
                })
        })
        .ok_or("source has no claimable spare with that principal")?;
    let entry = |phase| Asset::Transfer(id, destination, deadline, phase);
    if let Asset::Transfer(_, owner, original, phase) = before {
        if owner != destination || deadline != original {
            return Err("spare is bound to another transfer approval".into());
        }
        if phase == Confirmed {
            return Ok(Confirmed);
        }
    }
    let reported = canic::method::call::<HandoffStatus>(destination, 10, &id)
        .await
        .map_err(|e| format!("destination reservation unconfirmed: {e:?}"))??;
    super::authorize_change(caller)?;
    if before == entry(ControllersIssued) {
        if matches!(reported, Asset::AwaitingHandoff(subject) if subject == id) {
            return Ok(ControllersIssued);
        }
        match reported {
            Asset::Quarantined(p, _)
            | Asset::Available(p, _)
            | Asset::Claimed(p, _)
            | Asset::Released(p, _)
            | Asset::Reset(p, ..)
            | Asset::Transfer(p, ..)
                if p == id => {}
            _ => return Err("destination has not verified custody".into()),
        }
        replace(index, &before, entry(Confirmed))?;
        policy::wake();
        return Ok(Confirmed);
    }
    let check_deadline = || {
        let now = api::time();
        if deadline <= now || deadline - now > 600_000_000_000 {
            return Err("transfer requires the original deadline within ten minutes");
        }
        Ok(())
    };
    check_deadline()?;
    if before != entry(Reserved) {
        if reported != Asset::AwaitingRelease(id) {
            return Err("destination reservation already released".into());
        }
        if policy::subnet(api::canister_self()).await? != policy::subnet(destination).await? {
            return Err("destination Root is on another subnet".into());
        }
        super::authorize_change(caller)?;
        check_deadline()?;
        replace(index, &before, entry(Reserved))?;
        policy::wake();
    }
    let released = canic::method::call::<ImportStandby>(destination, 10, &id)
        .await
        .map_err(|e| format!("destination release unconfirmed: {e:?}"))??;
    if released != Asset::AwaitingHandoff(id) {
        return Err("destination did not retain release".into());
    }
    let args = mgmt::CanisterArgs { canister_id: id }
        .encode(256)
        .expect("principal");
    let (status, _) = reset::observe_target(&args).await?;
    if status.module_hash.is_some()
        || status.status != ic_platform_protocol::management_snapshots::Status::Running
    {
        return Err("transfer requires a running, uninstalled spare".into());
    }
    super::authorize_change(caller)?;
    check_deadline()?;
    let args = mgmt::UpdateSettingsArgs {
        canister_id: id,
        settings: mgmt::CanisterSettings {
            controllers: Some(vec![destination]),
            ..Default::default()
        },
        sender_canister_version: None,
    }
    .encode(4096)
    .expect("bounded controllers");
    replace(index, &entry(Reserved), entry(ControllersIssued))?;
    management("update_settings", args, 0)
        .await
        .map_err(|e| format!("controller handoff issued; observe destination custody: {e:?}"))?;
    Ok(ControllersIssued)
}
