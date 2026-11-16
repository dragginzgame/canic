//! Serial child upgrades with bounded immutable history and one active effect owner.
use super::{
    Artifact, MANAGEMENT_LIMITS, OBSERVING, STATE, State, delivery, failure, fault, management,
    observing, reserved_cycles, response, schedule, snapshot,
};
use canic_management_protocol::children::ChildId;
use canic_management_protocol::upgrade::{
    Approval, MAX_UPGRADES, Phase as UpgradePhase, Stage as UpgradeStage, Status as UpgradeStatus,
};
use canic_management_protocol::{Failure, Phase, Refusal, digest, stored};
use ic_platform::principal::Principal;
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{
        CanisterIdRecord, CanisterInstallMode, CanisterStatusResult, CanisterStatusType,
        InstallChunkedCodeArgs,
    },
};
use ic_platform_runtime::{
    call::{Call, Wait},
    system as api,
};

/// Read committed upgrade evidence without dispatching work.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn status(slot: ChildId) -> UpgradeStatus {
    STATE.with_borrow(|state| {
        state.as_ref().expect("initialized manager").journal.get().0[&slot]
            .1
            .clone()
    })
}
fn mutate(slot: ChildId, f: impl FnOnce(&mut UpgradeStatus)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut value = state.journal.get().clone();
        f(&mut value.0.get_mut(&slot).expect("known child").1);
        state.journal.set(value);
    });
}
/// Lookup never dispatches, replaces or reopens a historical operation.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn observe(slot: ChildId, expected: &Approval) -> UpgradeStatus {
    STATE.with_borrow(|state| {
        let journal = &state.as_ref().expect("initialized manager").journal.get().0[&slot];
        journal
            .2
            .iter()
            .chain(std::iter::once(&journal.1))
            .find(|record| record.approval.as_ref() == Some(expected))
            .cloned()
            .unwrap_or_default()
    })
}

pub(super) fn validate(state: &State, slot: ChildId) {
    let journal = &state.journal.get().0[&slot];
    assert!(journal.2.len() < MAX_UPGRADES, "bounded upgrade history");
    let mut previous = journal
        .0
        .approval
        .as_ref()
        .map_or([0; 32], |a| a.module_hash);
    let mut prior: Option<&UpgradeStatus> = None;
    for (index, record) in journal
        .2
        .iter()
        .chain(std::iter::once(&journal.1))
        .enumerate()
    {
        validate_record(record, &journal.0, &previous);
        if record.phase == UpgradePhase::Ready
            && let Artifact::Application(_, lifecycle) = &state.artifact.get().0[&slot]
        {
            assert_eq!(
                lifecycle.upgrades[index].phase,
                canic_management_protocol::application::MaintenancePhase::Active,
                "application readiness requires explicit activation"
            );
        }
        if let Some(old) = prior {
            assert_eq!(
                old.phase,
                UpgradePhase::Ready,
                "only verified completion admits a successor"
            );
            assert!(old.failure.is_none());
            assert!(
                record.approval.as_ref().expect("successor").execute_at_ns
                    > old.approval.as_ref().expect("predecessor").execute_at_ns,
                "ordered approvals"
            );
            if let Some(version) = record.previous_version {
                assert!(
                    version >= old.observed_version.expect("completed predecessor"),
                    "version regression"
                );
            }
        }
        if let Some(a) = &record.approval {
            previous = a.module_hash;
        }
        prior = Some(record);
    }
    if let Some(a) = &journal.1.approval {
        assert_eq!(
            state.artifact.get().0[&slot].hash(true),
            Some(a.module_hash),
            "current artifact custody"
        );
    } else {
        assert!(state.artifact.get().0[&slot].hash(true).is_none());
    }
}

fn validate_record(
    s: &UpgradeStatus,
    creation: &canic_management_protocol::Status,
    previous: &[u8; 32],
) {
    assert!(
        s.attempts.iter().all(|n| *n <= 1),
        "single-use upgrade counts"
    );
    for i in 0..2 {
        assert_eq!(
            s.attempts[i] == 1,
            s.requests[i].is_some(),
            "upgrade request custody"
        );
    }
    if let Some(a) = &s.approval {
        assert_eq!(creation.phase, Phase::Ready, "creation must be complete");
        assert_eq!(
            creation.worker.as_ref(),
            Some(&a.child),
            "upgrade child binding"
        );
        assert_eq!(a.manager, api::canister_self().to_text(), "upgrade owner");
        assert_eq!(&a.previous_module, previous, "old code");
        assert_ne!(a.previous_module, a.module_hash, "different executable");
        assert!(a.execute_at_ns < a.expires_at_ns, "upgrade interval");
        assert_ne!(s.phase, UpgradePhase::Empty, "admitted upgrade");
        if matches!(s.phase, UpgradePhase::StopIssued | UpgradePhase::Stopped) {
            assert_eq!(s.attempts, [0; 2], "stop precedes upload and installation");
            assert!(s.previous_version.is_none() && s.observed_version.is_none());
        }
        if s.attempts[0] == 1 {
            assert!(s.previous_version.is_some(), "old version custody");
        }
        if s.attempts[1] == 1 {
            assert_eq!(s.attempts[0], 1, "upload precedes upgrade");
        }
        if matches!(
            s.phase,
            UpgradePhase::Verified
                | UpgradePhase::Ready
                | UpgradePhase::StartIssued
                | UpgradePhase::Started
        ) {
            assert_eq!(s.attempts, [1; 2]);
            assert_eq!(
                s.observed_version,
                s.previous_version.and_then(|v| v.checked_add(1))
            );
            assert!(s.observed_version.is_some());
        }
    } else {
        assert_eq!(s, &UpgradeStatus::default(), "unadmitted upgrade evidence");
    }
}

/// Admit a successor only after Ready, retaining all earlier approvals and outcomes.
/// # Errors
/// Refuses incomplete creation, changed authority, invalid bytes/time or inadequate reserve.
/// # Panics
/// Traps for noncontrollers, uninitialized storage or failed durable writes.
pub fn stage(slot: ChildId, input: UpgradeStage) -> Result<UpgradeStatus, Refusal> {
    stage_input(slot, Input::Inline(input))
}
pub(super) enum Input {
    Inline(UpgradeStage),
    Stored(stored::Stage<Approval>),
    #[cfg(feature = "blob")]
    Application(canic_management_protocol::application::Upgrade),
}
impl Input {
    fn approval(&self) -> &Approval {
        match self {
            Self::Inline(s) => &s.approval,
            Self::Stored(s) => &s.approval,
            #[cfg(feature = "blob")]
            Self::Application(s) => &s.source.source.approval,
        }
    }
    fn validate(
        &self,
        manager: &str,
        child: &str,
        previous: &[u8; 32],
        now: u64,
    ) -> Result<(), Refusal> {
        match self {
            #[cfg(feature = "blob")]
            Self::Application(input) => {
                if input.attempt == 0 || input.source.arguments.as_slice().is_empty() {
                    return Err(Refusal::Binding);
                }
                input
                    .source
                    .source
                    .approval
                    .validate(manager, child, previous, now)?;
                delivery::admit(
                    &input.source.source.reference,
                    &input.source.source.approval.module_hash,
                )
            }
            Self::Inline(input) => input.validate(manager, child, previous, now),
            Self::Stored(input) => {
                input.approval.validate(manager, child, previous, now)?;
                delivery::admit(&input.reference, &input.approval.module_hash)
            }
        }
    }
}
pub(super) fn stage_input(slot: ChildId, input: Input) -> Result<UpgradeStatus, Refusal> {
    assert!(api::is_controller(&api::caller()), "controller required");
    if super::store_maintenance::active() {
        return Err(Refusal::Occupied);
    }
    let approval = input.approval().clone();
    #[cfg(feature = "blob")]
    let application = matches!(&input, Input::Application(_));
    #[cfg(not(feature = "blob"))]
    let application = false;
    if delivery::is_stored(slot) == matches!(&input, Input::Inline(_))
        || super::application::lifecycle(slot).is_some() != application
    {
        return Err(Refusal::Binding);
    }
    let retained = observe(slot, &approval);
    if retained.approval.is_some() {
        let matches = match &input {
            #[cfg(feature = "blob")]
            Input::Application(input) => super::application::upgrade_matches(slot, input),
            Input::Inline(input) => digest(&input.wasm) == approval.module_hash,
            Input::Stored(input) => super::stored_api::upgrade_source(slot, &approval)
                .is_some_and(|source| source.reference == input.reference),
        };
        return if matches {
            Ok(retained)
        } else {
            Err(Refusal::Artifact)
        };
    }
    let current = status(slot);
    if OBSERVING.with_borrow(|active| active.contains(&slot))
        || (current.approval.is_some()
            && (current.phase != UpgradePhase::Ready || current.failure.is_some()))
    {
        return Err(Refusal::Occupied);
    }
    let full = STATE.with_borrow(|state| {
        let journal = &state.as_ref().expect("initialized manager").journal.get().0[&slot];
        journal.2.len() + usize::from(journal.1.approval.is_some()) >= MAX_UPGRADES
    });
    if full {
        return Err(Refusal::Occupied);
    }
    let creation = snapshot(slot);
    if creation.phase != Phase::Ready {
        return Err(Refusal::Binding);
    }
    let previous = current.approval.as_ref().map_or_else(
        || {
            creation
                .approval
                .as_ref()
                .expect("creation approval")
                .module_hash
        },
        |a| a.module_hash,
    );
    if current
        .approval
        .as_ref()
        .is_some_and(|old| approval.execute_at_ns <= old.execute_at_ns)
    {
        return Err(Refusal::Occupied);
    }
    input.validate(
        &api::canister_self().to_text(),
        creation.worker.as_deref().expect("created child"),
        &previous,
        api::time(),
    )?;
    if ic_platform_runtime::raw::liquid_cycle_balance() < reserved_cycles() {
        return Err(Refusal::Reserve);
    }
    let artifacts = STATE.with_borrow(|state| {
        let mut artifacts = state
            .as_ref()
            .expect("initialized manager")
            .artifact
            .get()
            .clone();
        match (artifacts.0.get_mut(&slot).expect("known child"), input) {
            #[cfg(feature = "blob")]
            (Artifact::Application(sources, lifecycle), Input::Application(input)) => {
                use canic_management_protocol::application::{Maintenance, MaintenancePhase};
                sources.upgrades.push(stored::Delivery {
                    reference: input.source.source.reference.clone(),
                    operation: input.pin(slot).operation,
                    state: stored::PinState::Pending,
                    cache_deadline: None,
                });
                lifecycle.upgrades.push(Maintenance {
                    operator: api::caller().to_text(),
                    arguments: input.source.arguments,
                    attempt: input.attempt,
                    phase: MaintenancePhase::Scheduled,
                    requests: [None; 4],
                    before: None,
                    after: None,
                    activator: None,
                });
            }
            (Artifact::Inline(artifact), Input::Inline(input)) => artifact.1 = input.wasm,
            (Artifact::Stored(sources), Input::Stored(input)) => sources
                .upgrades
                .push(delivery::retained(slot, stored::STAGE_UPGRADE, &input)),
            _ => unreachable!("source mode admitted"),
        }
        artifacts
    });
    if !artifacts.fits() {
        return Err(Refusal::Artifact);
    }
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("initialized manager")
            .artifact
            .set(artifacts);
    });
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut journals = state.journal.get().clone();
        let journal = journals.0.get_mut(&slot).expect("known child");
        if journal.1.approval.is_some() {
            journal.2.push(journal.1.clone());
        }
        journal.1 = UpgradeStatus {
            approval: Some(approval),
            phase: UpgradePhase::Scheduled,
            ..Default::default()
        };
        state.journal.set(journals);
    });
    schedule(slot);
    Ok(status(slot))
}
pub(super) fn next(slot: ChildId) -> Option<u64> {
    let s = status(slot);
    #[cfg(feature = "blob")]
    if super::blob::waiting(slot) {
        return None;
    }
    if s.failure.is_some() {
        return None;
    }
    match s.phase {
        UpgradePhase::Scheduled => Some(s.approval.expect("approval").execute_at_ns),
        UpgradePhase::Checked
        | UpgradePhase::Stopped
        | UpgradePhase::Started
        | UpgradePhase::Uploaded
        | UpgradePhase::Installed
        | UpgradePhase::Verified => Some(api::time()),
        _ => None,
    }
}
pub(super) async fn inspect(a: &Approval) -> Result<CanisterStatusResult, Failure> {
    let args = candid_codec::encode_one(
        &CanisterIdRecord {
            canister_id: Principal::from_text(&a.child).map_err(|_| Failure::Mismatch)?,
        },
        1024,
    )
    .expect("status args");
    let reply = management("canister_status", args, 0).await?;
    let value: CanisterStatusResult =
        decoder::decode_one(&reply, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)?;
    if value.settings.controllers != [Principal::from_slice(api::canister_self().as_slice())] {
        return Err(Failure::Mismatch);
    }
    Ok(value)
}
fn issue(slot: ChildId, index: usize, phase: UpgradePhase, bytes: &[u8]) {
    mutate(slot, |s| {
        assert_eq!(s.attempts[index], 0, "no effect replay");
        s.attempts[index] = 1;
        s.requests[index] = Some(digest(bytes));
        s.phase = phase;
    });
}
pub(super) async fn step(slot: ChildId) {
    let s = status(slot);
    let Some(a) = &s.approval else {
        return;
    };
    if matches!(
        s.phase,
        UpgradePhase::Scheduled
            | UpgradePhase::Stopped
            | UpgradePhase::Checked
            | UpgradePhase::Uploaded
    ) {
        if api::time() > a.expires_at_ns {
            mutate(slot, |s| s.phase = UpgradePhase::Expired);
            return;
        }
        if api::time() < a.execute_at_ns {
            return;
        }
        if ic_platform_runtime::raw::liquid_cycle_balance() < reserved_cycles() {
            mutate(slot, |s| s.phase = UpgradePhase::ReserveBlocked);
            return;
        }
    }
    if s.phase == UpgradePhase::Scheduled && delivery::needs_pin(slot, true) {
        mutate(slot, |s| s.failure = Some(Failure::Uncertain));
        let result = delivery::pin(slot, true).await;
        mutate(slot, |s| s.failure = result.err());
        return; // Retention commits before subsequent inspection or installation.
    }
    #[cfg(feature = "blob")]
    if s.phase == UpgradePhase::Scheduled && super::application::lifecycle(slot).is_some() {
        match super::blob::prepare(slot, a).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                mutate(slot, |s| s.failure = Some(error));
                return;
            }
        }
    }
    let result = match s.phase {
        UpgradePhase::Scheduled if super::application::lifecycle(slot).is_none() => {
            let result = inspect(a).await;
            match result {
                Ok(value) if value.module_hash.as_deref() == Some(a.previous_module.as_slice()) => {
                    runtime_transition(slot, a, false).await
                }
                Ok(_) => Err(Failure::Mismatch),
                Err(error) => Err(error),
            }
        }
        UpgradePhase::Scheduled | UpgradePhase::Stopped => inspect(a).await.and_then(|value| {
            if value.module_hash.as_deref() != Some(a.previous_module.as_slice())
                || value.status != CanisterStatusType::Stopped
                || value.version == u64::MAX
                || STATE.with_borrow(|state| {
                    state.as_ref().expect("initialized manager").journal.get().0[&slot]
                        .2
                        .last()
                        .is_some_and(|old| {
                            old.observed_version
                                .is_none_or(|version| value.version < version)
                        })
                })
            {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| {
                s.previous_version = Some(value.version);
                s.phase = UpgradePhase::Checked;
            });
            Ok(())
        }),
        UpgradePhase::Checked => {
            let upload = delivery::upload(slot, true, a.module_hash, a.expires_at_ns);
            issue(slot, 0, UpgradePhase::UploadIssued, &upload.bytes);
            let result = upload
                .send()
                .await
                .map(|()| mutate(slot, |s| s.phase = UpgradePhase::Uploaded));
            fault(slot, 6); // Lost upload callback is recoverable from the chunk store.
            result
        }
        UpgradePhase::Uploaded => {
            match inspect(a).await {
                Err(error) => Err(error),
                Ok(value)
                    if value.module_hash.as_deref() != Some(a.previous_module.as_slice())
                        || Some(value.version) != s.previous_version
                        || value.status != CanisterStatusType::Stopped =>
                {
                    Err(Failure::Mismatch)
                }
                Ok(_) => {
                    // Inspection awaits: check the original deadline/reserve again at dispatch.
                    if api::time() > a.expires_at_ns {
                        mutate(slot, |s| s.phase = UpgradePhase::Expired);
                        return;
                    }
                    if ic_platform_runtime::raw::liquid_cycle_balance() < reserved_cycles() {
                        mutate(slot, |s| s.phase = UpgradePhase::ReserveBlocked);
                        return;
                    }
                    let (store, chunks) = delivery::layout(slot, true, a.module_hash);
                    let bytes = candid_codec::encode_one(
                        &InstallChunkedCodeArgs {
                            mode: CanisterInstallMode::Upgrade(None),
                            target_canister: Principal::from_text(&a.child)
                                .expect("retained child"),
                            store_canister: Some(store),
                            chunk_hashes_list: chunks,
                            wasm_module_hash: a.module_hash.to_vec(),
                            arg: super::application::arguments(slot, true),
                            sender_canister_version: Some(api::canister_version()),
                        },
                        8192,
                    )
                    .expect("upgrade args");
                    issue(slot, 1, UpgradePhase::InstallIssued, &bytes);
                    fault(slot, 7); // Trap rolls back dispatch and intent together.
                    let result =
                        management("install_chunked_code", bytes, 0)
                            .await
                            .and_then(|bytes| {
                                decoder::decode_args::<()>(
                                    &bytes,
                                    MANAGEMENT_LIMITS,
                                    decoder::Arity::Exact,
                                )
                                .map_err(|_| Failure::InvalidReply)?;
                                mutate(slot, |s| s.phase = UpgradePhase::Installed);
                                Ok(())
                            });
                    fault(slot, 8); // The upgrade may commit while its callback evidence rolls back.
                    result
                }
            }
        }
        UpgradePhase::Verified if super::application::lifecycle(slot).is_none() => {
            runtime_transition(slot, a, true).await
        }
        UpgradePhase::Installed | UpgradePhase::Verified | UpgradePhase::Started => {
            observation(slot).await
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        mutate(slot, |s| s.failure = Some(error));
    }
}
// Stop/start have no caller-supplied payload beyond the immutable approved child.
// Their issued phases are durable fences. No observation path can reissue them.
async fn runtime_transition(slot: ChildId, a: &Approval, start: bool) -> Result<(), Failure> {
    if api::time() > a.expires_at_ns
        || ic_platform_runtime::raw::liquid_cycle_balance() < reserved_cycles()
    {
        return Err(Failure::Uncertain);
    }
    let args = candid_codec::encode_one(
        &CanisterIdRecord {
            canister_id: Principal::from_text(&a.child).expect("approved child"),
        },
        1024,
    )
    .expect("bounded management target");
    mutate(slot, |s| {
        s.phase = if start {
            UpgradePhase::StartIssued
        } else {
            UpgradePhase::StopIssued
        };
        s.failure = Some(Failure::Uncertain);
    });
    let reply = management(
        if start {
            "start_canister"
        } else {
            "stop_canister"
        },
        args,
        0,
    )
    .await?;
    fault(slot, if start { 25 } else { 24 });
    decoder::decode_args::<()>(&reply, MANAGEMENT_LIMITS, decoder::Arity::Exact)
        .map_err(|_| Failure::InvalidReply)?;
    observe_runtime(slot, a, start).await
}
async fn observe_runtime(slot: ChildId, a: &Approval, start: bool) -> Result<(), Failure> {
    let value = inspect(a).await?;
    let (hash, state) = if start {
        (a.module_hash, CanisterStatusType::Running)
    } else {
        (a.previous_module, CanisterStatusType::Stopped)
    };
    if value.module_hash.as_deref() != Some(hash.as_slice()) || value.status != state {
        return Err(Failure::Mismatch);
    }
    // Starting increments the version, and subsequent application execution can
    // increment it further. Keep the earlier installation observation unchanged.
    if start
        && status(slot)
            .observed_version
            .is_none_or(|installed| value.version <= installed)
    {
        return Err(Failure::Mismatch);
    }
    mutate(slot, |s| {
        s.phase = if start {
            UpgradePhase::Started
        } else {
            UpgradePhase::Stopped
        };
        s.failure = None;
    });
    Ok(())
}
async fn observation(slot: ChildId) -> Result<(), Failure> {
    let s = status(slot);
    let Some(a) = &s.approval else {
        return Ok(());
    };
    #[cfg(feature = "blob")]
    if super::application::lifecycle(slot).is_some() {
        super::blob::observe(slot, a).await?;
        mutate(slot, |s| s.failure = None);
    }
    match s.phase {
        UpgradePhase::StopIssued => observe_runtime(slot, a, false).await?,
        UpgradePhase::StartIssued => observe_runtime(slot, a, true).await?,
        UpgradePhase::Scheduled if delivery::needs_pin(slot, true) => {
            if delivery::source(slot, true).expect("Store source").state == stored::PinState::Issued
            {
                delivery::pin(slot, true).await?;
                mutate(slot, |s| s.failure = None);
            }
        }
        UpgradePhase::UploadIssued => {
            delivery::observe_cache(slot, true, a.module_hash).await?;
            mutate(slot, |s| {
                s.phase = UpgradePhase::Uploaded;
                s.failure = None;
            });
        }
        UpgradePhase::InstallIssued | UpgradePhase::Installed => {
            let value = inspect(a).await?;
            if value.module_hash.as_deref() != Some(a.module_hash.as_slice())
                || Some(value.version) != s.previous_version.and_then(|v| v.checked_add(1))
            {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| {
                s.phase = UpgradePhase::Verified;
                s.observed_version = Some(value.version);
                s.failure = None;
            });
        }
        UpgradePhase::Verified if super::application::lifecycle(slot).is_none() => {}
        UpgradePhase::Verified | UpgradePhase::Started | UpgradePhase::Ready => {
            #[cfg(feature = "blob")]
            if super::application::lifecycle(slot).is_some()
                && !super::blob::resume(slot, a).await?
            {
                return Ok(());
            }
            let readiness =
                STATE.with_borrow(|state| state.as_ref().expect("state").readiness.clone());
            let reply = Call::new(
                ic_platform_types::Principal::from_slice(
                    (Principal::from_text(&a.child).expect("retained child")).as_slice(),
                ),
                readiness.method,
                Wait::Bounded { seconds: 5 },
            )
            .owned_args(readiness.request)
            .execute()
            .await
            .map_err(failure)
            .and_then(response)?;
            (readiness.accept)(&reply)?;
            mutate(slot, |s| {
                s.phase = UpgradePhase::Ready;
                s.failure = None;
            });
        }
        _ => {}
    }
    Ok(())
}
/// Recover observations and resume only effects that were never issued.
/// # Panics
/// Traps for noncontrollers, mismatched approvals, uninitialized storage or failed durable writes.
pub async fn reconcile(
    slot: ChildId,
    expected: canic_management_protocol::upgrade::Approval,
) -> UpgradeStatus {
    assert!(api::is_controller(&api::caller()), "controller required");
    assert!(
        !delivery::is_stored(slot),
        "Store-bound reconciliation required"
    );
    reconcile_bound(slot, expected).await
}
pub(super) async fn reconcile_bound(slot: ChildId, expected: Approval) -> UpgradeStatus {
    assert!(
        !super::store_maintenance::active(),
        "Store maintenance active"
    );
    let retained = observe(slot, &expected);
    assert_eq!(
        retained.approval.as_ref(),
        Some(&expected),
        "reconciliation approval mismatch"
    );
    if status(slot).approval.as_ref() != Some(&expected) {
        return retained;
    }
    let Some(guard) = observing(slot) else {
        return status(slot);
    };
    if let Err(error) = observation(slot).await {
        mutate(slot, |s| s.failure = Some(error));
    }
    drop(guard);
    super::resume_after_reconciliation(slot);
    status(slot)
}
