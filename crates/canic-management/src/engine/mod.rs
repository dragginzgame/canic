use crate::Readiness;
use crate::storage::{ARTIFACT_BYTES, Artifact, Artifacts, JOURNAL_BYTES, Journals};
use canic_management_protocol::children::{self, ChildId};
use canic_management_protocol::{
    Approval, CREATION_CYCLES, Failure, Phase, RESERVE_CYCLES, Refusal, Stage, Status, digest,
    stored,
};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{
        CanisterIdRecord, CanisterInstallMode, CanisterSettings, CanisterStatusResult,
        CreateCanisterArgs, InstallChunkedCodeArgs,
    },
};
use ic_platform_runtime::{
    Principal,
    call::{Call, CallFailed, WaitPolicy},
    cdk::api,
};
use ic_storage::{
    AllocationGrants, DefaultMemoryImpl, GenericAllocationPolicy, MemoryAuthority, MemoryRequest,
    MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig, adapter::StableCell,
};
use ic_timers::{Lifetime, Observation, Once, OnceDemand, OnceNext, OnceResult, OnceStatus};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
mod delivery;
pub(super) mod metrics;
pub(crate) mod store_maintenance;
pub(crate) mod stored_api;

pub(crate) mod upgrade;

/// Read all retained approvals without scheduling or dispatching work.
/// # Panics
/// Traps when lifecycle has not opened the engine.
#[must_use]
pub fn custody() -> canic_management_protocol::custody::Custody {
    let [first, second] = owner_custody();
    assert!(
        second.creation.approval.is_none(),
        "two-child custody required"
    );
    first
}
/// Read complete owner custody, including empty slots.
/// # Panics
/// Traps when lifecycle has not opened the engine.
#[must_use]
pub fn owner_custody() -> children::Custody {
    assert!(
        store_maintenance::observe().is_none(),
        "Store maintenance custody required"
    );
    assert!(
        !ChildId::ALL.into_iter().any(delivery::is_stored),
        "Store-aware custody required"
    );
    journals()
}
fn journals() -> children::Custody {
    STATE.with_borrow(|state| {
        let journals = &state.as_ref().expect("open engine").journal.get().0;
        ChildId::ALL.map(|slot| {
            let journal = &journals[&slot];
            let mut upgrades = journal.2.clone();
            if journal.1.approval.is_some() {
                upgrades.push(journal.1.clone());
            }
            canic_management_protocol::custody::Custody {
                creation: journal.0.clone(),
                upgrades,
            }
        })
    })
}

const MAX_PAGES: u64 = 64;
const MANAGEMENT_LIMITS: decoder::Limits = decoder::Limits {
    bytes: 16_384,
    header: 4096,
    types: 256,
    work: 65_536,
    allocation: 65_536,
};

struct State {
    journal: StableCell<Journals>,
    artifact: StableCell<Artifacts>,
    readiness: Readiness,
    timer_window: (u64, u64),
    _runtime: MemoryRuntime<DefaultMemoryImpl>,
}
thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static TIMER: RefCell<BTreeMap<ChildId, Once>> = const { RefCell::new(BTreeMap::new()) };
    static OBSERVING: RefCell<BTreeSet<ChildId>> = const { RefCell::new(BTreeSet::new()) };
    #[cfg(feature = "fault-injection")]
    static FAULT: RefCell<BTreeMap<ChildId, u8>> = const { RefCell::new(BTreeMap::new()) };
}

/// Open the bounded child engine and register independent continuation timers.
/// The canister owns lifecycle entrypoints and supplies its application readiness check.
/// Existing storage keys and wire evidence are retained across this extraction.
///
/// # Panics
/// Traps if storage is missing/corrupt on upgrade, admission fails, or timers cannot open.
pub fn open(fresh: bool, readiness: Readiness) {
    let mut runtime = MemoryRuntime::new_with_config(
        DefaultMemoryImpl::default(),
        StorageConfig {
            allocations: 2,
            extents: 3,
            pages: MAX_PAGES,
        },
    )
    .expect("admit manager backing");
    let allocations = runtime.diagnostics().expect("integrity").allocations;
    assert!(
        if fresh {
            allocations == 0
        } else {
            allocations == 2
        },
        "missing manager stores"
    );
    runtime
        .bootstrap(
            &SealedDeclarationSnapshot::new(vec![
                MemoryRequest::new("manager", "manager.journal.v1").expect("constant key"),
                MemoryRequest::new("manager", "manager.artifact.v1").expect("constant key"),
            ])
            .expect("constant declarations"),
            &AllocationGrants::new(vec![
                MemoryAuthority::new("manager", "manager.").expect("constant owner"),
            ])
            .expect("constant grants"),
            &GenericAllocationPolicy,
        )
        .expect("commit manager admission");
    let journal = runtime
        .open_memory(&StableKey::parse("manager.journal.v1").expect("constant key"))
        .expect("journal lease");
    let artifact = runtime
        .open_memory(&StableKey::parse("manager.artifact.v1").expect("constant key"))
        .expect("artifact lease");
    let journal = if fresh {
        StableCell::create(journal, Journals::default())
    } else {
        StableCell::load(journal, JOURNAL_BYTES)
    }
    .expect("strict journal opening");
    let artifact = if fresh {
        StableCell::create(artifact, Artifacts::default())
    } else {
        StableCell::load(artifact, ARTIFACT_BYTES)
    }
    .expect("strict artifact opening");
    let mut state = State {
        journal,
        artifact,
        readiness,
        timer_window: (api::canister_version(), api::time()),
        _runtime: runtime,
    };
    for slot in ChildId::ALL {
        validate(&state, slot);
        upgrade::validate(&state, slot);
        delivery::validate(&state, slot);
    }
    assert!(state.artifact.get().fits(), "aggregate artifact budget");
    store_maintenance::validate(&state);
    let mut children = BTreeSet::new();
    for child in state
        .journal
        .get()
        .0
        .values()
        .filter_map(|j| j.0.worker.as_ref())
    {
        assert!(children.insert(child), "distinct child identities");
    }
    // Commit identified collections only after strict validation of both old/new stores.
    // A post-upgrade trap rolls back both writes with the executable change.
    state.journal.set(state.journal.get().clone());
    state.artifact.set(state.artifact.get().clone());
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    ic_timers::initialize().expect("one timer runtime");
    for slot in ChildId::ALL {
        schedule(slot);
    }
}

fn validate(state: &State, slot: ChildId) {
    let s = &state.journal.get().0[&slot].0;
    assert!(s.attempts.iter().all(|n| *n <= 1), "single-use counts");
    for index in 0..3 {
        assert_eq!(
            s.attempts[index] == 1,
            s.requests[index].is_some(),
            "request custody"
        );
    }
    if let Some(a) = &s.approval {
        assert_eq!(
            a.manager,
            api::canister_self().to_text(),
            "manager identity"
        );
        assert_eq!(a.creation_cycles, CREATION_CYCLES, "ceiling");
        assert_eq!(
            state.artifact.get().0[&slot].hash(false),
            Some(a.module_hash),
            "artifact custody"
        );
        assert_ne!(s.phase, Phase::Empty, "admitted phase");
        assert!(
            a.expires_at_ns > a.execute_at_ns
                && a.expires_at_ns - a.execute_at_ns <= 600_000_000_000,
            "deadline"
        );
    } else {
        assert_eq!(s, &Status::default(), "unadmitted journal");
        assert!(
            state.artifact.get().0[&slot].hash(false).is_none(),
            "unadmitted artifact"
        );
    }
    if let Some(worker) = &s.worker {
        let worker = Principal::from_text(worker).expect("worker principal");
        assert!(
            worker != Principal::anonymous()
                && worker != Principal::management_canister()
                && worker != api::canister_self()
        );
        assert_eq!(s.attempts[0], 1);
        assert!(s.creation_refund.is_some());
    }
    if matches!(
        s.phase,
        Phase::Created
            | Phase::UploadIssued
            | Phase::Uploaded
            | Phase::InstallIssued
            | Phase::Installed
            | Phase::Verified
            | Phase::Ready
    ) {
        assert!(s.worker.is_some(), "retained worker required");
    }
    if s.attempts[1] == 1 {
        assert!(s.worker.is_some());
    }
    if s.attempts[2] == 1 {
        assert_eq!(s.attempts[1], 1);
    }
}

fn snapshot(slot: ChildId) -> Status {
    STATE.with_borrow(|state| {
        state.as_ref().expect("initialized manager").journal.get().0[&slot]
            .0
            .clone()
    })
}
fn mutate(slot: ChildId, f: impl FnOnce(&mut Status)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut journal = state.journal.get().clone();
        f(&mut journal.0.get_mut(&slot).expect("known child").0);
        state.journal.set(journal);
    });
}
fn next(s: &Status) -> Option<u64> {
    if s.failure.is_some() {
        return None;
    }
    match s.phase {
        Phase::Scheduled => Some(s.approval.as_ref().expect("approval").execute_at_ns),
        Phase::Created | Phase::Uploaded | Phase::Installed | Phase::Verified => Some(api::time()),
        _ => None,
    }
}
fn schedule(slot: ChildId) {
    let desired = next(&snapshot(slot))
        .or_else(|| upgrade::next(slot))
        .map_or(OnceDemand::Inactive, OnceDemand::At);
    TIMER.with_borrow_mut(|timers| {
        let timer = timers.entry(slot).or_insert_with(|| {
            ic_timers::bind_once(
                [
                    "profile",
                    "manager",
                    match slot {
                        ChildId::FIRST => "one-worker",
                        ChildId::SECOND => "second-worker",
                        _ => unreachable!("unsupported child timer identity"),
                    },
                ],
                // Child slots keep their identities across creation and all upgrades.
                Lifetime::Retained,
                move || async move {
                    let Some(observation) = observing(slot) else {
                        return OnceResult {
                            observation: Observation::NoWork,
                            next: OnceNext::Stop,
                        };
                    };
                    if snapshot(slot).phase == Phase::Ready {
                        upgrade::step(slot).await;
                    } else {
                        step(slot).await;
                    }
                    drop(observation);
                    let decision = next(&snapshot(slot))
                        .or_else(|| upgrade::next(slot))
                        .map_or(OnceNext::Stop, OnceNext::At);
                    OnceResult {
                        observation: if snapshot(slot).failure.is_some()
                            || upgrade::status(slot).failure.is_some()
                        {
                            Observation::ExpectedFailure
                        } else {
                            Observation::Success
                        },
                        next: decision,
                    }
                },
            )
            .expect("bind retained child timer")
        });
        timer.set(desired).expect("schedule retained demand");
    });
}

// Only authenticated domain reconciliation may clear an abandoned timer's
// barrier. Stop it first, then derive eligibility from the retained journal;
// these local commands commit together and never reset effect intent.
fn resume_after_reconciliation(slot: ChildId) {
    TIMER.with_borrow(|timers| {
        if let Some(timer) = timers.get(&slot)
            && let OnceStatus::RecoveryRequired(token) = timer.status().expect("timer status")
        {
            timer
                .recover(token, OnceDemand::Inactive)
                .expect("reconciled timer recovery");
        }
    });
    schedule(slot);
}
/// Read committed evidence without resuming work.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn status(slot: ChildId) -> Status {
    let mut value = snapshot(slot);
    value.stable_pages = api::stable_size();
    value.maintenance_candidate = cfg!(feature = "maintenance-candidate");
    value
}
/// Retain one exact controller-approved artifact before scheduling effects.
/// # Errors
/// Refuses changed authority, invalid artifact/binding/time or inadequate reserve.
/// # Panics
/// Traps for noncontrollers, uninitialized storage or failed durable writes.
pub fn stage(slot: ChildId, input: Stage) -> Result<Status, Refusal> {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    if store_maintenance::active() || delivery::is_stored(slot) {
        return Err(Refusal::Occupied);
    }
    if let Some(previous) = snapshot(slot).approval {
        return if previous == input.approval && digest(&input.wasm) == previous.module_hash {
            Ok(status(slot))
        } else {
            Err(Refusal::Occupied)
        };
    }
    input.validate(&api::canister_self().to_text(), api::time())?;
    admit(
        slot,
        input.approval,
        Artifact::Inline(crate::storage::Inline(input.wasm, vec![])),
    )
}
fn admit(slot: ChildId, approval: Approval, artifact: Artifact) -> Result<Status, Refusal> {
    if api::canister_liquid_cycle_balance() < reserved_cycles() + u128::from(CREATION_CYCLES) {
        return Err(Refusal::Reserve);
    }
    let mut artifacts =
        STATE.with_borrow(|state| state.as_ref().expect("open engine").artifact.get().clone());
    *artifacts.0.get_mut(&slot).expect("known child") = artifact;
    if !artifacts.fits() {
        return Err(Refusal::Artifact);
    }
    STATE.with_borrow_mut(|state| state.as_mut().expect("open engine").artifact.set(artifacts));
    mutate(slot, |s| {
        s.approval = Some(approval);
        s.phase = Phase::Scheduled;
    });
    schedule(slot);
    Ok(status(slot))
}

fn fault(slot: ChildId, point: u8) {
    #[cfg(feature = "fault-injection")]
    if FAULT.with_borrow(|faults| faults.get(&slot).copied()) == Some(point) {
        api::trap("injected growth boundary");
    }
    #[cfg(not(feature = "fault-injection"))]
    let _ = (slot, point);
}
#[cfg(feature = "fault-injection")]
/// Select a test-only interrupted-message boundary.
/// # Panics
/// Traps unless the caller controls this canister and the point is in range.
pub fn inject_fault(slot: ChildId, value: u8) {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    assert!(value <= 19);
    FAULT.with_borrow_mut(|faults| {
        if value == 0 {
            faults.remove(&slot);
        } else {
            faults.insert(slot, value);
        }
    });
}

// Endpoint faults fail the real Wire encoder after the handler returns: staging
// is synchronous, while reconciliation can already have committed an outgoing call.
#[cfg(feature = "fault-injection")]
pub(crate) fn reply_fault(method: u64) -> bool {
    matches!(
        (
            FAULT.with_borrow(|faults| faults.get(&ChildId::FIRST).copied()),
            method
        ),
        (Some(9), canic_management_protocol::STAGE)
            | (Some(10), canic_management_protocol::RECONCILE)
            | (Some(12), canic_management_protocol::publication::EXECUTE)
            | (Some(12), canic_management_protocol::publication::INFO)
            | (Some(12), canic_management_protocol::publication::CUSTODY)
            | (Some(12), canic_management_protocol::publication::ADMISSION)
    )
}

fn failure(error: CallFailed) -> Failure {
    match error {
        CallFailed::CallRejected(rejection) => Failure::Rejected(rejection.raw_reject_code()),
        _ => Failure::Uncertain,
    }
}
async fn management(method: &str, bytes: Vec<u8>, cycles: u128) -> Result<Vec<u8>, Failure> {
    Call::new(
        Principal::management_canister(),
        method,
        WaitPolicy::Unbounded,
    )
    .take_raw_args(bytes)
    .with_cycles(cycles)
    .execute()
    .await
    .map(|reply| reply.to_vec())
    .map_err(failure)
}
fn issue(slot: ChildId, index: usize, phase: Phase, bytes: &[u8]) {
    mutate(slot, |s| {
        assert_eq!(s.attempts[index], 0);
        s.attempts[index] = 1;
        s.requests[index] = Some(digest(bytes));
        s.phase = phase;
        s.failure = None;
    });
}
fn worker(s: &Status) -> Principal {
    Principal::from_text(s.worker.as_ref().expect("retained principal")).expect("valid principal")
}

async fn step(slot: ChildId) {
    let s = snapshot(slot);
    if matches!(s.phase, Phase::Scheduled | Phase::Created | Phase::Uploaded) {
        let a = s.approval.as_ref().expect("approval");
        if api::time() > a.expires_at_ns {
            mutate(slot, |s| s.phase = Phase::Expired);
            return;
        }
        if api::time() < a.execute_at_ns {
            return;
        }
        if api::canister_liquid_cycle_balance() < reserved_cycles() {
            mutate(slot, |s| s.phase = Phase::ReserveBlocked);
            return;
        }
    }
    match s.phase {
        Phase::Scheduled => {
            if delivery::needs_pin(slot, false) {
                mutate(slot, |s| s.failure = Some(Failure::Uncertain));
                let result = delivery::pin(slot, false).await;
                mutate(slot, |s| s.failure = result.err());
                fault(slot, 11); // Lost pin callback is recovered from Store custody.
                return; // Commit retention evidence before any creation dispatch.
            }
            let bytes = candid_codec::encode_one(
                &CreateCanisterArgs {
                    settings: Some(CanisterSettings {
                        controllers: Some(vec![api::canister_self()]),
                        ..Default::default()
                    }),
                    sender_canister_version: Some(api::canister_version()),
                },
                4096,
            )
            .expect("create args");
            issue(slot, 0, Phase::CreateIssued, &bytes);
            fault(slot, 1); // Trap rolls back intent and prevents dispatch; original demand survives.
            let reply = management("create_canister", bytes, u128::from(CREATION_CYCLES)).await;
            let refunded = api::msg_cycles_refunded();
            match reply.and_then(|bytes| {
                decoder::decode_one::<CanisterIdRecord>(&bytes, MANAGEMENT_LIMITS)
                    .map_err(|_| Failure::InvalidReply)
            }) {
                Ok(value) => mutate(slot, |s| {
                    s.worker = Some(value.canister_id.to_text());
                    s.creation_refund = Some(refunded);
                    s.phase = Phase::Created;
                }),
                Err(error) => mutate(slot, |s| {
                    s.failure = Some(error);
                    s.creation_refund = Some(refunded);
                }),
            }
            fault(slot, 2); // Creation-result loss must remain blocked, including across upgrade.
        }
        Phase::Created => {
            fault(slot, 3); // A later message cannot erase the already committed principal.
            let approval = s.approval.as_ref().expect("approval");
            let upload =
                delivery::upload(slot, false, approval.module_hash, approval.expires_at_ns);
            issue(slot, 1, Phase::UploadIssued, &upload.bytes);
            match upload.send().await {
                Ok(()) => mutate(slot, |s| s.phase = Phase::Uploaded),
                Err(error) => mutate(slot, |s| s.failure = Some(error)),
            }
            fault(slot, 4);
        }
        Phase::Uploaded => {
            let hash = s.approval.as_ref().expect("approval").module_hash.to_vec();
            let (store, chunks) = delivery::layout(
                slot,
                false,
                s.approval.as_ref().expect("approval").module_hash,
            );
            let bytes = candid_codec::encode_one(
                &InstallChunkedCodeArgs {
                    mode: CanisterInstallMode::Install,
                    target_canister: worker(&s),
                    store_canister: Some(store),
                    chunk_hashes_list: chunks,
                    wasm_module_hash: hash,
                    arg: vec![],
                    sender_canister_version: Some(api::canister_version()),
                },
                4096,
            )
            .expect("install args");
            issue(slot, 2, Phase::InstallIssued, &bytes);
            let result = management("install_chunked_code", bytes, 0)
                .await
                .and_then(|bytes| {
                    decoder::decode_args::<()>(&bytes, MANAGEMENT_LIMITS, decoder::Arity::Exact)
                        .map_err(|_| Failure::InvalidReply)
                });
            match result {
                Ok(()) => mutate(slot, |s| s.phase = Phase::Installed),
                Err(error) => mutate(slot, |s| s.failure = Some(error)),
            }
            fault(slot, 5);
        }
        Phase::Installed | Phase::Verified => {
            if let Err(error) = observation(slot).await {
                mutate(slot, |s| s.failure = Some(error));
            }
        }
        _ => {}
    }
}

async fn observation(slot: ChildId) -> Result<(), Failure> {
    let before = snapshot(slot);
    let Some(a) = before.approval.as_ref() else {
        return Ok(());
    };
    match before.phase {
        Phase::Scheduled if delivery::needs_pin(slot, false) => {
            if delivery::source(slot, false).expect("Store source").state
                == stored::PinState::Issued
            {
                delivery::pin(slot, false).await?;
                mutate(slot, |s| s.failure = None);
            }
        }
        Phase::UploadIssued => {
            delivery::observe_cache(slot, false, a.module_hash).await?;
            mutate(slot, |s| {
                s.phase = Phase::Uploaded;
                s.failure = None;
            });
        }
        Phase::InstallIssued | Phase::Installed => {
            let bytes = candid_codec::encode_one(
                &CanisterIdRecord {
                    canister_id: worker(&before),
                },
                1024,
            )
            .expect("status args");
            let reply = management("canister_status", bytes, 0).await?;
            let value: CanisterStatusResult = decoder::decode_one(&reply, MANAGEMENT_LIMITS)
                .map_err(|_| Failure::InvalidReply)?;
            if value.module_hash.as_deref() != Some(a.module_hash.as_slice())
                || value.settings.controllers != [api::canister_self()]
            {
                return Err(Failure::Mismatch);
            }
            mutate(slot, |s| {
                s.phase = Phase::Verified;
                s.observed_module = value.module_hash;
                s.failure = None;
            });
        }
        Phase::Verified | Phase::Ready => {
            let readiness =
                STATE.with_borrow(|state| state.as_ref().expect("state").readiness.clone());
            let bytes = readiness.request;
            let reply = Call::new(
                worker(&before),
                readiness.method,
                WaitPolicy::Bounded { timeout_seconds: 5 },
            )
            .take_raw_args(bytes)
            .execute()
            .await
            .map_err(failure)?;
            (readiness.accept)(&reply)?;
            mutate(slot, |s| {
                s.phase = Phase::Ready;
                s.failure = None;
            });
        }
        _ => {} // No observation can invent a lost creation principal.
    }
    Ok(())
}
/// Reconstruct observable outcomes without reissuing uncertain effects.
/// # Panics
/// Traps for noncontrollers, mismatched approvals, uninitialized storage or failed durable writes.
pub async fn reconcile(slot: ChildId, expected: canic_management_protocol::Approval) -> Status {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    assert!(
        !delivery::is_stored(slot),
        "Store-bound reconciliation required"
    );
    reconcile_bound(slot, expected).await
}
async fn reconcile_bound(slot: ChildId, expected: Approval) -> Status {
    assert!(!store_maintenance::active(), "Store maintenance active");
    assert_eq!(
        snapshot(slot).approval,
        Some(expected),
        "reconciliation approval mismatch"
    );
    let Some(guard) = observing(slot) else {
        return status(slot);
    };
    if let Err(error) = observation(slot).await {
        mutate(slot, |s| s.failure = Some(error));
    }
    drop(guard);
    resume_after_reconciliation(slot);
    status(slot)
}

// Pending, unissued attachments are non-overlapping reservations. Issued calls have
// already deducted their cycles; uncertain outcomes never reissue that attachment.
fn reserved_cycles() -> u128 {
    STATE.with_borrow(|state| {
        u128::from(RESERVE_CYCLES)
            + state
                .as_ref()
                .expect("open engine")
                .journal
                .get()
                .0
                .values()
                .filter(|journal| journal.0.phase == Phase::Scheduled)
                .map(|_| u128::from(CREATION_CYCLES))
                .sum::<u128>()
    })
}
// The executor drops the suspended future during callback-trap cleanup. Keep
// this heap exclusion owned by that future; durable effect intent stays intact.
fn observing(slot: ChildId) -> Option<impl Drop> {
    if !OBSERVING.with_borrow_mut(|active| active.insert(slot)) {
        return None;
    }
    Some(std::mem::DropGuard::new(slot, |slot| {
        OBSERVING.with_borrow_mut(|active| {
            active.remove(&slot);
        });
    }))
}
