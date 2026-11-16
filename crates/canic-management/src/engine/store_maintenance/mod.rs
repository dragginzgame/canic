//! Root retains Store maintenance intent and evidence; Host retains replacement bytes.
use super::{
    MANAGEMENT_LIMITS, OBSERVING, STATE, State, api, candid_codec, decoder, digest, management,
};
use canic_management_protocol::{
    Failure, Refusal,
    store_maintenance::{Approval, MAX_UPGRADES, Phase, Scan, Status},
};
use ic_platform::management_canister_types::{
    CanisterIdRecord, CanisterStatusResult, CanisterStatusType,
};
use ic_platform::principal::Principal;
use ic_platform_runtime::call::{Call, Wait};
use ic_wasm_store_protocol as store;
mod installation;
mod preservation;
pub use installation::{activate, install};

pub(crate) fn active() -> bool {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("open engine")
            .journal
            .get()
            .1
            .as_ref()
            .is_some_and(|record| record.phase != Phase::Ready)
    })
}
fn quiescent(state: &State) -> bool {
    state.journal.get().0.values().all(|journal| {
        matches!(
            journal.0.phase,
            canic_management_protocol::Phase::Empty | canic_management_protocol::Phase::Ready
        ) && journal.0.failure.is_none()
            && (journal.1.approval.is_none()
                || journal.1.phase == canic_management_protocol::upgrade::Phase::Ready)
            && journal.1.failure.is_none()
    })
}
fn binding(state: &State, approval: &Approval) -> bool {
    let Ok(target) = Principal::from_text(&approval.store) else {
        return false;
    };
    target.to_text() == approval.store
        && target != Principal::anonymous()
        && target != Principal::management_canister()
        && target != Principal::from_slice(api::canister_self().as_slice())
        && approval.previous_module != [0; 32]
        && approval.module_hash != [0; 32]
        && approval.previous_module != approval.module_hash
        && approval.attempt.sequence != 0
        && approval.attempt.operation != [0; 32]
        && state
            .journal
            .get()
            .0
            .values()
            .all(|j| j.0.worker.as_ref() != Some(&approval.store))
        && state.artifact.get().0.values().all(|a| {
            a.sources()
                .is_none_or(|s| s.creation.reference.store == approval.store)
        })
}
pub(super) fn validate(state: &State) {
    let journals = state.journal.get();
    assert!(journals.2.len() < MAX_UPGRADES, "Store history bound");
    assert!(
        journals.2.is_empty() || journals.1.is_some(),
        "Store history tip"
    );
    let records: Vec<_> = journals.2.iter().chain(journals.1.iter()).collect();
    for (index, record) in records.iter().enumerate() {
        if index < journals.2.len() {
            assert!(
                record.phase == Phase::Ready && record.failure.is_none(),
                "completed Store predecessor"
            );
        }
        if index > 0 {
            assert!(
                successor(records[index - 1], &record.approval),
                "Store maintenance chain"
            );
        }
        assert!(
            records[..index]
                .iter()
                .all(|old| old.approval.attempt.operation != record.approval.attempt.operation),
            "distinct Store operation"
        );
        assert!(
            binding(state, &record.approval) && (record.phase == Phase::Ready || quiescent(state)),
            "maintenance owner binding"
        );
        let operator = Principal::from_text(&record.operator).expect("maintenance operator");
        assert_eq!(operator.to_text(), record.operator);
        assert_ne!(operator, Principal::anonymous());
        preservation::validate(record);
        installation::validate(record);
        assert_eq!(
            record.stop_request.is_some(),
            !matches!(
                record.phase,
                Phase::Reserved | Phase::Closing | Phase::Closed
            )
        );
        if let Some(request) = record.stop_request {
            assert_eq!(
                request,
                digest(&args(&record.approval)),
                "retained stop request"
            );
        }
    }
}

/// Observe Root's retained Store intent while Store is unavailable; never resumes work.
/// # Panics
/// Traps if lifecycle has not opened the engine.
#[must_use]
pub fn observe() -> Option<Status> {
    STATE.with_borrow(|state| state.as_ref().expect("open engine").journal.get().1.clone())
}
fn successor(previous: &Status, approval: &Approval) -> bool {
    previous.phase == Phase::Ready
        && previous.failure.is_none()
        && previous.approval.store == approval.store
        && previous.approval.module_hash == approval.previous_module
        && previous.approval.attempt.sequence.checked_add(1) == Some(approval.attempt.sequence)
}

/// Read one exact retained approval without advancing or replacing any operation.
/// # Panics
/// Traps if lifecycle has not opened the engine.
#[must_use]
pub fn observe_attempt(approval: &Approval) -> Option<Status> {
    STATE.with_borrow(|state| {
        let journals = state.as_ref().expect("open engine").journal.get();
        journals
            .1
            .iter()
            .chain(journals.2.iter())
            .find(|record| &record.approval == approval)
            .cloned()
    })
}

/// Fence new child/publication mutations and retain one exact Store-maintenance approval.
/// # Errors
/// Refuses unfinished child work, changed authority/bindings, or an invalid dispatch interval.
/// # Panics
/// Traps for noncontrollers, uninitialized storage or failed durable writes.
pub fn stage(approval: Approval) -> Result<Status, Refusal> {
    let operator = api::caller();
    assert!(api::is_controller(&operator), "controller required");
    // Reopen requires a captured authenticated operator; reject before retaining unusable custody.
    if operator == ic_platform_types::Principal::anonymous() {
        return Err(Refusal::Binding);
    }
    if let Some(old) = observe_attempt(&approval) {
        return if old.operator == operator.to_text() {
            Ok(old)
        } else {
            Err(Refusal::Occupied)
        };
    }
    // Occupied authority dominates a changed request's deadline, as in the original contract.
    if STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("open engine")
            .journal
            .get()
            .1
            .as_ref()
            .is_some_and(|old| !successor(old, &approval))
    }) {
        return Err(Refusal::Occupied);
    }
    let now = api::time();
    if approval.expires_at_ns <= now || approval.expires_at_ns - now > 3_600_000_000_000 {
        return Err(Refusal::Time);
    }
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open engine");
        if !binding(state, &approval) {
            return Err(Refusal::Binding);
        }
        if !quiescent(state) || OBSERVING.with_borrow(|active| !active.is_empty()) {
            return Err(Refusal::Occupied);
        }
        let journals = state.journal.get();
        if journals.2.len() + usize::from(journals.1.is_some()) >= MAX_UPGRADES
            || (journals.1.is_none() && approval.attempt.sequence != 1)
            || journals
                .1
                .iter()
                .chain(journals.2.iter())
                .any(|old| old.approval.attempt.operation == approval.attempt.operation)
        {
            return Err(Refusal::Occupied);
        }
        let record = Status {
            approval,
            operator: operator.to_text(),
            phase: Phase::Reserved,
            stop_request: None,
            failure: None,
            preservation: Scan::Entries(None),
            installation: None,
        };
        let mut journals = state.journal.get().clone();
        if let Some(previous) = journals.1.replace(record.clone()) {
            journals.2.push(previous);
        }
        state.journal.set(journals);
        Ok(record)
    })
}
fn selected(approval: &Approval) -> Status {
    let value = observe_attempt(approval).expect("retained Store maintenance");
    assert_eq!(&value.approval, approval, "maintenance approval mismatch");
    assert_eq!(
        value.operator,
        api::caller().to_text(),
        "original operator required"
    );
    assert!(api::is_controller(&api::caller()), "controller required");
    value
}
fn change(approval: &Approval, expected: Phase, update: impl FnOnce(&mut Status)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open engine");
        let mut journals = state.journal.get().clone();
        let value = journals.1.as_mut().expect("retained maintenance");
        // Both operation and phase fence callbacks, including a successor at the same phase.
        if &value.approval == approval && value.phase == expected {
            update(value);
            state.journal.set(journals);
        }
    });
}
fn dispatch(record: &Status) -> Result<(), Failure> {
    let current = observe().expect("retained maintenance");
    if current.approval != record.approval
        || current.phase != record.phase
        || !api::is_controller(
            &ic_platform_types::Principal::from_text(&record.operator).expect("retained operator"),
        )
    {
        return Err(Failure::Uncertain);
    }
    if api::time() > record.approval.expires_at_ns {
        return Err(Failure::Mismatch);
    }
    if ic_platform_runtime::raw::liquid_cycle_balance()
        < u128::from(canic_management_protocol::RESERVE_CYCLES)
    {
        return Err(Failure::Uncertain);
    }
    Ok(())
}
fn target(approval: &Approval) -> Principal {
    Principal::from_text(&approval.store).expect("retained Store")
}
fn args(approval: &Approval) -> Vec<u8> {
    candid_codec::encode_one(
        &CanisterIdRecord {
            canister_id: target(approval),
        },
        1024,
    )
    .expect("stop args")
}
async fn inspect(approval: &Approval) -> Result<CanisterStatusResult, Failure> {
    inspect_module(approval, approval.previous_module).await
}
async fn inspect_module(
    approval: &Approval,
    module: [u8; 32],
) -> Result<CanisterStatusResult, Failure> {
    let reply = management("canister_status", args(approval), 0).await?;
    let value: CanisterStatusResult =
        decoder::decode_one(&reply, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)?;
    if value.module_hash.as_deref() != Some(module.as_slice())
        || value.settings.controllers != [Principal::from_slice(api::canister_self().as_slice())]
    {
        return Err(Failure::Mismatch);
    }
    Ok(value)
}
async fn gate(approval: &Approval, command: store::AdmissionCommand) -> Result<(), Failure> {
    admission(approval, command, true).await
}
fn admission_frame(command: &store::AdmissionCommand) -> Vec<u8> {
    ic_wire::encode(
        command,
        ic_wire::Encoding::Frame {
            method: store::ADMISSION,
            direction: ic_wire::Direction::Request,
        },
        store::LIMITS,
    )
    .expect("admission request")
}
async fn admission(
    approval: &Approval,
    command: store::AdmissionCommand,
    closed: bool,
) -> Result<(), Failure> {
    let bytes = admission_frame(&command);
    let reply = Call::new(
        ic_platform_types::Principal::from_slice((target(approval)).as_slice()),
        "admission",
        Wait::Bounded { seconds: 5 },
    )
    .owned_args(bytes)
    .execute()
    .await
    .map_err(super::failure)
    .and_then(super::response)?;
    let result: Result<Option<store::Admission>, store::Error> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: store::ADMISSION,
            direction: ic_wire::Direction::Response,
        },
        store::LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)?;
    let expected = store::Admission {
        attempt: approval.attempt.clone(),
        owner: Principal::from_slice(api::canister_self().as_slice())
            .as_slice()
            .to_vec(),
        closed,
    };
    if result != Ok(Some(expected)) {
        return Err(Failure::Mismatch);
    }
    Ok(())
}

/// Advance the exact retained operation; issued effects are recovered by observation only.
/// Installation and activation require their separate explicit requests.
/// # Panics
/// Traps for mismatched authority, invalid lifecycle, injected faults or failed durable writes.
pub async fn advance(approval: Approval) -> Status {
    run_step(selected(&approval)).await
}

/// Retain observed outcomes of already-issued effects without starting a new effect.
/// The captured phase selects one observation branch even if another call advances concurrently.
/// # Panics
/// Traps for mismatched authority, invalid lifecycle or failed durable writes.
pub async fn recover(approval: Approval) -> Status {
    let record = selected(&approval);
    if matches!(
        record.phase,
        Phase::Closing
            | Phase::StopIssued
            | Phase::UploadIssued
            | Phase::InstallIssued
            | Phase::StartIssued
            | Phase::Verifying
            | Phase::ActivateIssued
    ) {
        run_step(record).await
    } else {
        record
    }
}

async fn run_step(record: Status) -> Status {
    let result = step(&record).await;
    if let Err(error) = result {
        change(&record.approval, record.phase, |value| {
            value.failure = Some(error)
        });
    }
    observe_attempt(&record.approval).expect("retained maintenance")
}
async fn step(record: &Status) -> Result<(), Failure> {
    let approval = &record.approval;
    match record.phase {
        Phase::Reserved => {
            if inspect(approval).await?.status != CanisterStatusType::Running {
                return Err(Failure::Mismatch);
            }
            dispatch(record)?;
            change(approval, Phase::Reserved, |value| {
                value.phase = Phase::Closing;
                value.failure = None;
            });
            // Intent and outgoing enqueue commit together; callback failure cannot erase the intent.
            let result = gate(
                approval,
                store::AdmissionCommand::Close(approval.attempt.clone()),
            )
            .await;
            super::fault(super::ChildId::FIRST, 13);
            match result {
                Ok(()) => change(approval, Phase::Closing, |value| {
                    value.phase = Phase::Closed;
                    value.failure = None;
                }),
                Err(error) => change(approval, Phase::Closing, |value| {
                    value.failure = Some(error)
                }),
            }
        }
        Phase::Closing => {
            gate(approval, store::AdmissionCommand::Observe).await?;
            change(approval, Phase::Closing, |value| {
                value.phase = Phase::Closed;
                value.failure = None;
            });
        }
        Phase::Closed => {
            if !record.preservation.as_ref().is_some_and(|p| p.complete) {
                return preservation::capture(record).await;
            }
            if inspect(approval).await?.status != CanisterStatusType::Running {
                return Err(Failure::Mismatch);
            }
            dispatch(record)?;
            let bytes = args(approval);
            change(approval, Phase::Closed, |value| {
                value.phase = Phase::StopIssued;
                value.stop_request = Some(digest(&bytes));
                value.failure = None;
            });
            let result = management("stop_canister", bytes, 0)
                .await
                .and_then(|reply| {
                    decoder::decode_args::<()>(&reply, MANAGEMENT_LIMITS, decoder::Arity::Exact)
                        .map_err(|_| Failure::InvalidReply)
                });
            super::fault(super::ChildId::FIRST, 14);
            if let Err(error) = result {
                change(approval, Phase::StopIssued, |value| {
                    value.failure = Some(error)
                });
            }
        }
        Phase::StopIssued | Phase::Stopped => {
            if inspect(approval).await?.status != CanisterStatusType::Stopped {
                return Err(Failure::Uncertain);
            }
            change(&record.approval, record.phase, |value| {
                value.phase = Phase::Stopped;
                value.failure = None;
            });
        }
        Phase::UploadIssued
        | Phase::Uploaded
        | Phase::InstallIssued
        | Phase::Installed
        | Phase::StartIssued
        | Phase::Verifying
        | Phase::Verified
        | Phase::ActivateIssued
        | Phase::Ready => installation::advance(record).await?,
    }
    Ok(())
}
