//! Optional Blob lifecycle around the existing child upgrade effect owner.
//! Supplier state decides admission. Issued calls are observed, never replaced.
use super::{Artifact, STATE, application, upgrade};
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
use ic_platform_protocol::management_snapshots::Status as RuntimeStatus;
use ic_platform_runtime::{
    call::{Call, Wait},
    system as api,
};
use ic_platform_types::Principal;

fn state(slot: ChildId) -> Option<Maintenance> {
    application::lifecycle(slot)?.upgrades.last().cloned()
}

fn mutate(slot: ChildId, f: impl FnOnce(&mut Maintenance)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open engine");
        let mut artifacts = state.artifact.clone();
        let Artifact::Application(_, lifecycle) = artifacts.0.get_mut(&slot).expect("slot") else {
            panic!("application custody")
        };
        f(lifecycle.upgrades.last_mut().expect("admitted maintenance"));
        assert!(artifacts.fits(), "maintenance evidence budget");
        state.set_artifact(artifacts);
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
        Principal::from_text(&a.child).expect("retained child"),
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
    let bytes = call(a, "canic_blob_management", frame(blob::MANAGEMENT, &())).await?;
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
        || view.configuration.manager.as_slice() != api::canister_self().as_slice()
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
    let bytes = call(a, "canic_blob_update", bytes).await?;
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
    upgrade::dispatch_runtime_transition(slot, a, start, if start { 22 } else { 21 }, |bytes| {
        issue(
            slot,
            if start {
                Phase::StartIssued
            } else {
                Phase::StopIssued
            },
            if start { 2 } else { 1 },
            bytes,
        );
    })
    .await?;
    observe(slot, a).await
}

/// Advance only unissued preparation. Each successful boundary commits before the next timer.
pub(super) async fn prepare(slot: ChildId, a: &Approval) -> Result<bool, Failure> {
    let s = state(slot).expect("maintenance");
    match s.phase {
        Phase::Scheduled => {
            let runtime = upgrade::inspect(a).await?;
            if runtime.module_hash.as_deref() != Some(&a.previous_module)
                || runtime.status != RuntimeStatus::Running
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
                        RuntimeStatus::Running
                    } else {
                        RuntimeStatus::Stopped
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
