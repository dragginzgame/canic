//! Optional Blob lifecycle around the existing child upgrade effect owner.
//! Supplier state decides admission. Issued calls are observed, never replaced.
use super::{Artifact, STATE, application, management, upgrade};
use canic::blob::{
    self,
    supplier::{
        local::Error,
        protocol::{Command, ManagementView, Outcome},
    },
};
use canic_management_protocol::{
    Failure,
    application::{Arguments, Maintenance, MaintenancePhase as Phase, Upgrade},
    children::ChildId,
    upgrade::Approval,
};
use ic_platform::principal::Principal;
use ic_platform::{
    candid_codec,
    management_canister_types::{CanisterIdRecord, CanisterStatusType},
};
use ic_platform_runtime::{
    call::{Call, Wait},
    system as api,
};

fn state(slot: ChildId) -> Option<Maintenance> {
    application::lifecycle(slot)?.upgrades.last().cloned()
}
fn mutate(slot: ChildId, f: impl FnOnce(&mut Maintenance)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open engine");
        let mut artifacts = state.artifact.get().clone();
        let Artifact::Application(_, lifecycle) = artifacts.0.get_mut(&slot).expect("slot") else {
            panic!("application custody")
        };
        f(lifecycle.upgrades.last_mut().expect("admitted maintenance"));
        assert!(artifacts.fits(), "maintenance evidence budget");
        state.artifact.set(artifacts);
    });
}
pub(super) fn waiting(slot: ChildId) -> bool {
    state(slot).is_some_and(|s| {
        matches!(
            s.phase,
            Phase::DrainIssued
                | Phase::StopIssued
                | Phase::StartIssued
                | Phase::Inspected
                | Phase::ActivateIssued
        )
    })
}
fn frame(method: u64, value: &impl serde::Serialize) -> Vec<u8> {
    ic_wire::encode(
        value,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        },
        blob::RPC_LIMITS,
    )
    .expect("bounded Blob control request")
}
async fn call(a: &Approval, method: &str, args: Vec<u8>) -> Result<Vec<u8>, Failure> {
    Call::new(
        ic_platform_types::Principal::from_slice(
            (Principal::from_text(&a.child).expect("retained child")).as_slice(),
        ),
        method,
        Wait::Bounded { seconds: 5 },
    )
    .owned_args(args)
    .execute()
    .await
    .map_err(super::failure)
    .and_then(super::response)
}
async fn inspect(a: &Approval) -> Result<(ManagementView, Arguments), Failure> {
    let bytes = call(a, "blob_management", frame(blob::MANAGEMENT, &())).await?;
    let view: Result<ManagementView, Error> = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: blob::MANAGEMENT,
            direction: ic_wire::Direction::Response,
        },
        blob::RPC_LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)?;
    let view = view.map_err(|_| Failure::Mismatch)?;
    let child = Principal::from_text(&a.child).expect("retained child");
    if view.installation.as_slice() != child.as_slice()
        || view.configuration.application.as_slice() != child.as_slice()
        || view.configuration.manager.as_slice()
            != Principal::from_slice(api::canister_self().as_slice()).as_slice()
    {
        return Err(Failure::Mismatch);
    }
    Ok((
        view,
        Arguments::new(bytes).map_err(|_| Failure::InvalidReply)?,
    ))
}
fn issue(slot: ChildId, phase: Phase, index: usize, bytes: &[u8]) {
    mutate(slot, |s| {
        assert!(s.requests[index].is_none(), "one maintenance dispatch");
        s.requests[index] = Some(canic_management_protocol::digest(bytes));
        if index == 3 {
            s.activator = Some(s.operator.clone());
        }
        s.phase = phase;
    });
}
async fn command(slot: ChildId, a: &Approval, activate: bool) -> Result<(), Failure> {
    let s = state(slot).expect("maintenance");
    let input = if activate {
        Command::Activate(s.attempt)
    } else {
        Command::Drain(s.attempt)
    };
    let bytes = frame(blob::UPDATE, &input);
    issue(
        slot,
        if activate {
            Phase::ActivateIssued
        } else {
            Phase::DrainIssued
        },
        if activate { 3 } else { 0 },
        &bytes,
    );
    let bytes = call(a, "blob_update", bytes).await?;
    super::fault(slot, if activate { 23 } else { 20 });
    let result: Result<Outcome, Error> = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: blob::UPDATE,
            direction: ic_wire::Direction::Response,
        },
        blob::RPC_LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)?;
    match result {
        Ok(Outcome::Maintenance(m)) if m.attempt == s.attempt && m.active == activate => {
            observe(slot, a).await
        }
        _ => Err(Failure::Mismatch),
    }
}
async fn runtime(slot: ChildId, a: &Approval, start: bool) -> Result<(), Failure> {
    // Restart is a new effect too; observing a completed installation cannot
    // renew its original deadline or consume the other children's reserve.
    if api::time() > a.expires_at_ns
        || ic_platform_runtime::raw::liquid_cycle_balance() < super::reserved_cycles()
    {
        return Err(Failure::Uncertain);
    }
    let bytes = candid_codec::encode_one(
        &CanisterIdRecord {
            canister_id: Principal::from_text(&a.child).expect("retained child"),
        },
        1024,
    )
    .expect("management args");
    issue(
        slot,
        if start {
            Phase::StartIssued
        } else {
            Phase::StopIssued
        },
        if start { 2 } else { 1 },
        &bytes,
    );
    let bytes = management(
        if start {
            "start_canister"
        } else {
            "stop_canister"
        },
        bytes,
        0,
    )
    .await?;
    super::fault(slot, if start { 22 } else { 21 });
    candid_codec::decoder::decode_args::<()>(
        &bytes,
        super::MANAGEMENT_LIMITS,
        candid_codec::decoder::Arity::Exact,
    )
    .map_err(|_| Failure::InvalidReply)?;
    observe(slot, a).await
}
/// Advance only unissued preparation. Each successful boundary commits before the next timer.
pub(super) async fn prepare(slot: ChildId, a: &Approval) -> Result<bool, Failure> {
    let s = state(slot).expect("maintenance");
    match s.phase {
        Phase::Scheduled => {
            let runtime = upgrade::inspect(a).await?;
            if runtime.module_hash.as_deref() != Some(&a.previous_module)
                || runtime.status != CanisterStatusType::Running
            {
                return Err(Failure::Mismatch);
            }
            let (view, _) = inspect(a).await?;
            if !view.maintenance.active
                || view.maintenance.attempt.checked_add(1) != Some(s.attempt)
            {
                return Err(Failure::Mismatch);
            }
            if api::time() > a.expires_at_ns {
                return Err(Failure::Mismatch);
            }
            command(slot, a, false).await?;
        }
        Phase::Drained => {
            if api::time() > a.expires_at_ns {
                return Err(Failure::Mismatch);
            }
            runtime(slot, a, false).await?;
        }
        Phase::Stopped => return Ok(true),
        _ => return Err(Failure::Uncertain),
    }
    Ok(false)
}
/// Reconstruct evidence only. No effect is authorized by missing/failed observations.
pub(super) async fn observe(slot: ChildId, a: &Approval) -> Result<(), Failure> {
    let s = state(slot).expect("maintenance");
    match s.phase {
        Phase::DrainIssued => {
            let (view, bytes) = inspect(a).await?;
            if view.maintenance.attempt != s.attempt || view.maintenance.active {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| {
                s.before = Some(bytes);
                s.phase = Phase::Drained;
            });
        }
        Phase::StopIssued | Phase::StartIssued => {
            let value = upgrade::inspect(a).await?;
            let start = s.phase == Phase::StartIssued;
            let hash = if start {
                &a.module_hash
            } else {
                &a.previous_module
            };
            if value.module_hash.as_deref() != Some(hash.as_slice())
                || value.status
                    != if start {
                        CanisterStatusType::Running
                    } else {
                        CanisterStatusType::Stopped
                    }
            {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| {
                s.phase = if start {
                    Phase::Started
                } else {
                    Phase::Stopped
                }
            });
        }
        Phase::ActivateIssued => {
            let (view, _) = inspect(a).await?;
            if view.maintenance.attempt != s.attempt || !view.maintenance.active {
                return Err(Failure::Mismatch);
            }
            let mut closed = view;
            closed.maintenance.active = false;
            let bytes = ic_wire::encode(
                &Ok::<_, Error>(closed),
                ic_wire::Encoding::Frame {
                    method: blob::MANAGEMENT,
                    direction: ic_wire::Direction::Response,
                },
                blob::RPC_LIMITS,
            )
            .expect("bounded management state");
            if s.before.as_ref().map(Arguments::as_slice) != Some(bytes.as_slice()) {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| s.phase = Phase::Active);
        }
        _ => {}
    }
    Ok(())
}
/// Start and inspect after the existing owner has verified the installed code/version.
pub(super) async fn resume(slot: ChildId, a: &Approval) -> Result<bool, Failure> {
    match state(slot).expect("maintenance").phase {
        Phase::Stopped => runtime(slot, a, true).await?,
        Phase::Started => {
            let (view, bytes) = inspect(a).await?;
            let s = state(slot).expect("maintenance");
            if view.maintenance.active
                || view.maintenance.attempt != s.attempt
                || s.before.as_ref() != Some(&bytes)
            {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| {
                s.after = Some(bytes);
                s.phase = Phase::Inspected;
            });
        }
        Phase::Active => return Ok(true),
        _ => {}
    }
    Ok(false)
}
/// Explicit activation under the original operation. This endpoint never upgrades or renews it.
/// # Panics
/// Traps for changed/unauthorized authority or failed durable writes.
pub async fn activate(slot: ChildId, input: Upgrade) -> Result<Maintenance, Failure> {
    assert!(api::is_controller(&api::caller()), "controller required");
    assert!(
        application::upgrade_matches(slot, &input),
        "complete maintenance binding"
    );
    assert_eq!(
        upgrade::status(slot).approval.as_ref(),
        Some(&input.source.source.approval),
        "current upgrade required"
    );
    let Some(_guard) = super::observing(slot) else {
        return Err(Failure::Uncertain);
    };
    let s = state(slot).expect("maintenance");
    assert_eq!(
        s.operator,
        api::caller().to_text(),
        "original operator required"
    );
    let a = &input.source.source.approval;
    match s.phase {
        Phase::Active => return Ok(s),
        Phase::ActivateIssued => observe(slot, a).await?,
        Phase::Inspected => {
            if api::time() > a.expires_at_ns
                || upgrade::status(slot).phase
                    != canic_management_protocol::upgrade::Phase::Verified
            {
                return Err(Failure::Mismatch);
            }
            let (view, bytes) = inspect(a).await?;
            if view.maintenance.active || s.after.as_ref() != Some(&bytes) {
                return Err(Failure::Mismatch);
            }
            if api::time() > a.expires_at_ns {
                return Err(Failure::Mismatch);
            }
            command(slot, a, true).await?;
        }
        _ => return Err(Failure::Mismatch),
    }
    drop(_guard);
    super::schedule(slot);
    Ok(state(slot).expect("maintenance"))
}
