pub(crate) mod application;
#[cfg(feature = "blob")]
pub(crate) mod blob;
mod delivery;
pub(crate) mod directory;
#[cfg(feature = "role-issuer")]
pub mod role_issuer;
#[cfg(feature = "sharding")]
pub mod sharding;
pub(crate) mod store_maintenance;
pub(crate) mod stored_api;

pub(crate) mod upgrade;

use crate::{
    Readiness,
    storage::{
        ARTIFACT_BYTES, ARTIFACT_KEY, Artifact, Artifacts, JOURNAL_BYTES, JOURNAL_KEY, Journals,
        KEYS,
    },
};
use canic_management_protocol::{
    Approval, CREATION_CYCLES, Failure, Phase, RESERVE_CYCLES, Refusal, Stage, Status,
    children::{self, ChildId},
    digest, stored,
};
use ic_platform_protocol::{
    candid::Limits,
    management::{self as mgmt, CanisterArgs, CanisterSettings, CreateCanisterArgs, InstallMode},
};
use ic_platform_runtime::{
    call::{Call, Completion, Outcome, SubmitError, Wait},
    system as api,
};
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, adapter::StableCell};
use ic_timers::{Lifetime, Observation, Once, OnceDemand, OnceNext, OnceResult, OnceStatus};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

/// Read all retained approvals without scheduling or dispatching work.
/// # Panics
/// Traps when lifecycle has not opened the engine.
#[must_use]
pub fn custody() -> canic_management_protocol::custody::Custody {
    let mut slots = owner_custody().into_iter();
    let first = slots.next().expect("first custody slot");
    assert!(
        slots.all(|s| s.creation.approval.is_none()),
        "two-child custody required"
    );
    first
}

/// Read complete owner custody, including empty slots.
/// # Panics
/// Traps when lifecycle has not opened the engine.
#[must_use]
pub fn owner_custody() -> children::Custody {
    application::require_legacy();
    assert!(
        store_maintenance::observe().is_none(),
        "Store maintenance custody required"
    );
    assert!(
        !service_slots().into_iter().any(delivery::is_stored),
        "Store-aware custody required"
    );
    journals()
}

pub(crate) fn service_slots() -> Vec<ChildId> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("open engine")
            .journal
            .0
            .keys()
            .copied()
            .filter(|id| id.service_index().is_some())
            .collect()
    })
}

/// Retain the fleet ceiling during a fresh Root installation.
/// # Panics
/// Traps if the engine is closed, the policy is invalid or bootstrap already ran.
pub fn bootstrap(policy: canic_management_protocol::stored::RootBootstrap) {
    assert!(policy.valid(), "invalid Root bootstrap policy");
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("initialized Root");
        let mut journals = state.journal.clone();
        assert!(journals.3.replace(policy).is_none(), "already bootstrapped");
        state.set_journal(journals);
    });
}

fn journals() -> children::Custody {
    STATE.with_borrow(|state| {
        let journals = &state.as_ref().expect("open engine").journal.0;
        service_slots()
            .into_iter()
            .map(|slot| {
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
            .collect()
    })
}

const MANAGEMENT_LIMITS: Limits = Limits {
    bytes: 16_384,
    header: 4096,
    types: 256,
    work: 65_536,
    allocation: 65_536,
    vector: 16_384,
    skipping: 65_536,
};

pub(crate) struct State {
    journal: Journals,
    artifact: Artifacts,
    journal_cell: StableCell<Vec<u8>>,
    artifact_cell: StableCell<Vec<u8>>,
    #[cfg(feature = "backup")]
    backup_cell: StableCell<Vec<u8>>,
    #[cfg(feature = "sharding")]
    pool: StableCell<Vec<u8>>,
    #[cfg(feature = "sharding")]
    pools: sharding::Pools,
    #[cfg(feature = "sharding")]
    worker_history: ic_storage::adapter::StableBTreeMap<[u8; 32], Vec<u8>>,
    #[cfg(feature = "role-issuer")]
    role_key: StableCell<Vec<u8>>,
    readiness: Readiness,
    #[cfg(feature = "backup")]
    pub(crate) backup: crate::backup::Ledger,
}
impl State {
    #[cfg(feature = "backup")]
    pub(crate) fn save_backup(&mut self) {
        let bytes = self.backup.encode();
        self.backup_cell.set(bytes);
    }

    fn set_journal(&mut self, journal: Journals) {
        let bytes = journal.to_bytes().expect("encode management journal");
        self.journal_cell.set(bytes.into_owned());
        self.journal = journal;
    }

    fn set_artifact(&mut self, artifact: Artifacts) {
        let bytes = artifact.to_bytes().expect("encode management artifacts");
        self.artifact_cell.set(bytes.into_owned());
        self.artifact = artifact;
    }
}

thread_local! {
    pub(crate) static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static TIMER: RefCell<BTreeMap<ChildId, Once>> = const { RefCell::new(BTreeMap::new()) };
    static OBSERVING: RefCell<BTreeSet<ChildId>> = const { RefCell::new(BTreeSet::new()) };
    #[cfg(feature = "fault-injection")]
    static FAULT: RefCell<BTreeMap<ChildId, u8>> = const { RefCell::new(BTreeMap::new()) };
}

/// Open the bounded child engine and register independent continuation timers.
/// The canister owns lifecycle entrypoints and supplies its application readiness check.
/// Reopens the records backing strictly; older allocator images require their old executable.
///
/// # Panics
/// Traps if storage is missing/corrupt on upgrade, admission fails, or timers cannot open.
pub fn open(fresh: bool, readiness: Readiness) {
    // Placement Roots also retain the registry index and worker history.
    let pages = if cfg!(feature = "sharding") {
        4096
    } else {
        512
    };
    let runtime =
        canic::storage::bootstrap(DefaultMemoryImpl::default(), fresh, "manager", KEYS, pages);
    let journal_cell = canic::storage::cell(
        &runtime,
        JOURNAL_KEY,
        JOURNAL_BYTES,
        fresh.then(|| {
            Journals::default()
                .to_bytes()
                .expect("journal")
                .into_owned()
        }),
    );
    let journal = Journals::from_bytes(journal_cell.get()).unwrap_or_else(|error| api::trap(error));
    let artifact_cell = canic::storage::cell(
        &runtime,
        ARTIFACT_KEY,
        ARTIFACT_BYTES,
        fresh.then(|| {
            Artifacts::default()
                .to_bytes()
                .expect("artifact")
                .into_owned()
        }),
    );
    #[cfg(feature = "backup")]
    let backup_cell = canic::storage::cell(
        &runtime,
        "manager.snapshot.v1",
        crate::backup::CAPACITY,
        fresh.then(|| crate::backup::Ledger::default().encode()),
    );
    #[cfg(feature = "sharding")]
    let pool = canic::storage::cell(
        &runtime,
        crate::storage::SHARDS_KEY,
        sharding::BYTES,
        fresh.then(|| sharding::Pools::default().encode()),
    );
    #[cfg(feature = "sharding")]
    let worker_history = {
        let lease = runtime
            .open_memory(&ic_storage::StableKey::parse(crate::storage::WORKER_HISTORY_KEY).unwrap())
            .expect("placement index lease");
        if fresh {
            ic_storage::adapter::StableBTreeMap::create(lease)
        } else {
            ic_storage::adapter::StableBTreeMap::load(lease)
        }
    };
    let mut state = State {
        journal,
        artifact: Artifacts::from_bytes(artifact_cell.get().as_slice().into())
            .expect("artifact custody"),
        #[cfg(feature = "role-issuer")]
        role_key: role_issuer::open(&runtime, fresh),
        journal_cell,
        #[cfg(feature = "sharding")]
        pools: sharding::open(pool.get()),

        #[cfg(feature = "sharding")]
        worker_history,
        #[cfg(feature = "sharding")]
        pool,
        artifact_cell,
        #[cfg(feature = "backup")]
        backup: crate::backup::Ledger::open(backup_cell.get()),
        #[cfg(feature = "backup")]
        backup_cell,
        readiness,
    };
    assert!(
        state.journal.0.keys().eq(state.artifact.0.keys()),
        "matching child custody"
    );
    let slots: Vec<_> = state.journal.0.keys().copied().collect();
    #[cfg(feature = "sharding")]
    sharding::validate_claims(&state);
    #[cfg(not(feature = "sharding"))]
    assert!(
        slots.iter().all(|s| s.service_index().is_some()),
        "project placement required"
    );
    for &slot in &slots {
        validate(&state, slot);
        upgrade::validate(&state, slot);
        delivery::validate(&state, slot);
    }
    assert!(state.artifact.fits(), "aggregate artifact budget");
    store_maintenance::validate(&state);
    let mut children = BTreeSet::new();
    for (slot, journal) in &state.journal.0 {
        #[cfg(feature = "sharding")]
        if state.pools.archived(*slot) {
            continue;
        }
        if let Some(child) = &journal.0.worker {
            assert!(
                children.insert(child),
                "distinct live child identities at {slot:?}"
            );
        }
    }
    // Normalize admitted artifact payloads only after validating complete custody.
    state.set_artifact(state.artifact.clone());
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    canic::lifecycle::initialize_diagnostics();
    canic::lifecycle::initialize_timers().expect("one timer runtime");
    for slot in slots {
        schedule(slot);
    }
    #[cfg(feature = "sharding")]
    {
        sharding::standby::policy::wake();
        sharding::funding::resume();
    }
    if !fresh {
        directory::schedule(None);
    }
}

fn validate(state: &State, slot: ChildId) {
    let s = &state.journal.0[&slot].0;
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
            state.artifact.0[&slot].hash(false),
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
            state.artifact.0[&slot].hash(false).is_none(),
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
        #[cfg(not(feature = "sharding"))]
        let created = true;
        #[cfg(feature = "sharding")]
        let created = state.pools.standby_claim(slot) != Some(worker);
        assert_eq!(s.attempts[0], u8::from(created));
        assert_eq!(s.creation_refund.is_some(), created);
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
        state
            .as_ref()
            .expect("initialized manager")
            .journal
            .0
            .get(&slot)
            .map(|j| j.0.clone())
            .unwrap_or_default()
    })
}

fn mutate(slot: ChildId, f: impl FnOnce(&mut Status)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut journal = state.journal.clone();
        let previous = &state.journal.0[&slot].0;
        let next = &mut journal.0.get_mut(&slot).expect("known child").0;
        f(next);
        if previous.phase != next.phase || previous.failure != next.failure {
            use canic::logging::{Level, record};
            let phase = next.phase;
            let id = slot.index();
            if let Some(reason) = &next.failure {
                record(
                    Level::Warn,
                    "management",
                    &format!("child {id} at {phase:?}: {reason:?}"),
                );
            } else if matches!(phase, Phase::Expired | Phase::ReserveBlocked | Phase::Ready) {
                let level = if phase == Phase::Ready {
                    Level::Info
                } else {
                    Level::Warn
                };
                record(level, "management", &format!("child {id}: {phase:?}"));
            }
        }
        state.set_journal(journal);
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
            let name = format!("child-{}", slot.index());
            ic_timers::bind_once(
                [
                    "profile",
                    "manager",
                    match slot {
                        ChildId::FIRST => "one-worker",
                        ChildId::SECOND => "second-worker",
                        s if s == ChildId::PROJECT[0] => "first-project",
                        s if s == ChildId::PROJECT[1] => "second-project",
                        _ => &name,
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
    value.stable_pages = ic_platform_runtime::raw::stable64_size();
    value.maintenance_candidate = cfg!(feature = "maintenance-candidate");
    value
}

/// Retain one exact controller-approved artifact before scheduling effects.
/// # Errors
/// Refuses changed authority, invalid artifact/binding/time or inadequate reserve.
/// # Panics
/// Traps for noncontrollers, uninitialized storage or failed durable writes.
pub fn stage(slot: ChildId, input: Stage) -> Result<Status, Refusal> {
    assert!(api::is_controller(&api::caller()), "controller required");
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
        Origin::Permanent,
        slot,
        input.approval,
        Artifact::Inline(crate::storage::Inline(input.wasm, vec![])),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Permanent,
    #[cfg(feature = "sharding")]
    Paid(sharding::Acquisition, u128),
}

fn admit(
    origin: Origin,
    slot: ChildId,
    approval: Approval,
    artifact: Artifact,
) -> Result<Status, Refusal> {
    if origin == Origin::Permanent
        && slot
            .service_index()
            .is_none_or(|index| index > service_slots().len())
    {
        return Err(Refusal::Binding);
    }
    if origin == Origin::Permanent
        && STATE.with_borrow(|s| {
            let state = s.as_ref().expect("open Root");
            #[cfg(feature = "sharding")]
            if state.pools.capacity_approved() {
                return false;
            }
            let maximum = state.journal.3.map_or(
                canic_management_protocol::stored::DEFAULT_SUBNET_CHILD_LIMIT,
                |policy| policy.max_children,
            );
            state
                .journal
                .0
                .values()
                .filter(|entry| entry.0.approval.is_some())
                .count()
                >= maximum as usize
        })
    {
        return Err(Refusal::Occupied);
    }
    #[cfg(not(feature = "sharding"))]
    let attachment = u128::from(CREATION_CYCLES);
    #[cfg(feature = "sharding")]
    let selected = match origin {
        Origin::Permanent => STATE.with_borrow(|state| {
            state
                .as_ref()
                .expect("open Root")
                .pools
                .acquisition()
                .map_err(|_| Refusal::Occupied)
        })?,
        Origin::Paid(acquisition, _) => acquisition,
    };
    #[cfg(feature = "sharding")]
    let attachment = u128::from(selected.attachment());
    #[cfg(feature = "sharding")]
    let generation = artifact
        .sources()
        .map_or(approval.module_hash, |s| s.creation.operation);
    let liquid = ic_platform_runtime::raw::liquid_cycle_balance();
    #[cfg(feature = "sharding")]
    let liquid = match origin {
        Origin::Paid(_, credit) => liquid.saturating_add(credit),
        _ => liquid,
    };
    if liquid < reserved_cycles() + attachment {
        return Err(Refusal::Reserve);
    }
    let mut artifacts =
        STATE.with_borrow(|state| state.as_ref().expect("open engine").artifact.clone());
    // The released four-child format opens both original project slots together.
    if ChildId::PROJECT.contains(&slot) {
        for original in ChildId::PROJECT {
            artifacts.0.entry(original).or_default();
        }
    }
    artifacts.0.insert(slot, artifact);
    if !artifacts.fits() {
        return Err(Refusal::Artifact);
    }
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open engine");
        state.journal.0.entry(slot).or_default();
        if ChildId::PROJECT.contains(&slot) {
            for original in ChildId::PROJECT {
                state.journal.0.entry(original).or_default();
            }
        }
        state.set_artifact(artifacts);
    });
    mutate(slot, |s| {
        s.approval = Some(approval);
        s.phase = Phase::Scheduled;
    });
    #[cfg(feature = "sharding")]
    sharding::retain_child(slot, generation, origin, selected);
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
    assert!(api::is_controller(&api::caller()), "controller required");
    assert!(value <= 26);
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
        (
            Some(9),
            canic_management_protocol::STAGE
                | canic_management_protocol::children::STAGE
                | canic_management_protocol::application::STAGE_CHILD_CREATION
        ) | (
            Some(10),
            canic_management_protocol::RECONCILE
                | canic_management_protocol::children::RECONCILE
                | canic_management_protocol::application::RECONCILE_CHILD_CREATION
        ) | (Some(12), canic_management_protocol::publication::EXECUTE)
            | (
                Some(12),
                canic_management_protocol::publication::OBSERVATION
            )
            | (Some(12), canic_management_protocol::publication::ADMISSION)
            | (Some(12), canic_management_protocol::publication::REMOVE)
    )
}

fn failure(_error: SubmitError) -> Failure {
    Failure::Uncertain
}

fn response(completion: Completion) -> Result<Vec<u8>, Failure> {
    match completion.into_parts().0 {
        Outcome::Reply(bytes) => Ok(bytes),
        Outcome::Rejected(rejection) => Err(Failure::Rejected(rejection.code())),
        Outcome::OutcomeUnknown(_) => Err(Failure::Uncertain),
    }
}

async fn management_completion(
    method: &str,
    bytes: Vec<u8>,
    cycles: u128,
) -> Result<Completion, SubmitError> {
    Call::new(
        ic_platform_types::Principal::management_canister(),
        method,
        Wait::Unbounded,
    )
    .owned_args(bytes)
    .cycles(cycles)
    .execute()
    .await
}

pub(crate) async fn deposit_cycles(
    child: ic_platform_types::Principal,
    amount: u64,
) -> (bool, Option<u128>) {
    use ic_platform_protocol::management as mgmt;
    let args = mgmt::CanisterArgs { canister_id: child }
        .encode(1024)
        .expect("bounded deposit request");
    let result = management_completion("deposit_cycles", args, amount.into()).await;
    let (success, refund) = match result {
        Ok(completion) => {
            let (outcome, refund) = completion.into_parts();
            let success = matches!(outcome, Outcome::Reply(ref bytes) if
                mgmt::decode_empty_response(bytes, MANAGEMENT_LIMITS).is_ok())
                && refund == Some(0);
            (success, refund)
        }
        Err(_) => (false, None),
    };
    (success, refund)
}

async fn management(method: &str, bytes: Vec<u8>, cycles: u128) -> Result<Vec<u8>, Failure> {
    management_completion(method, bytes, cycles)
        .await
        .map_err(failure)
        .and_then(response)
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

fn create_args() -> Vec<u8> {
    CreateCanisterArgs {
        settings: Some(CanisterSettings {
            controllers: Some(vec![api::canister_self()]),
            ..Default::default()
        }),
        sender_canister_version: Some(api::canister_version()),
    }
    .encode(4096)
    .expect("create args")
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
        if ic_platform_runtime::raw::liquid_cycle_balance() < reserved_cycles() {
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
            #[cfg(feature = "sharding")]
            if let Some(id) = sharding::standby::claimed(slot) {
                mutate(slot, |s| {
                    s.worker = Some(id.to_text());
                    s.phase = Phase::Created;
                });
                return;
            }
            let bytes = create_args();
            issue(slot, 0, Phase::CreateIssued, &bytes);
            fault(slot, 1); // Trap rolls back intent and prevents dispatch; original demand survives.
            let completion =
                management_completion("create_canister", bytes, u128::from(CREATION_CYCLES)).await;
            let refunded = completion
                .as_ref()
                .ok()
                .and_then(Completion::refunded_cycles);
            let reply = completion.map_err(failure).and_then(response);
            match reply.and_then(|bytes| {
                mgmt::decode_created(&bytes, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)
            }) {
                Ok(value) => mutate(slot, |s| {
                    s.worker = Some(value.canister_id.to_text());
                    s.creation_refund = refunded;
                    s.phase = Phase::Created;
                }),
                Err(error) => mutate(slot, |s| {
                    s.failure = Some(error);
                    s.creation_refund = refunded;
                }),
            }
            fault(slot, 2); // Creation-result loss must remain blocked, including across upgrade.
        }
        Phase::Created => {
            fault(slot, 3); // A later message cannot erase the already committed principal.
            let approval = s.approval.as_ref().expect("approval");
            match delivery::dispatch_upload(
                slot,
                false,
                approval.module_hash,
                approval.expires_at_ns,
                |bytes| issue(slot, 1, Phase::UploadIssued, bytes),
            )
            .await
            {
                Ok(()) => mutate(slot, |s| s.phase = Phase::Uploaded),
                Err(error) => mutate(slot, |s| s.failure = Some(error)),
            }
            fault(slot, 4);
        }
        Phase::Uploaded => {
            let result = delivery::install(
                slot,
                InstallMode::Install,
                worker(&s),
                s.approval.as_ref().expect("approval").module_hash,
                |bytes| issue(slot, 2, Phase::InstallIssued, bytes),
            )
            .await;
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

enum ChildObservation {
    Pin,
    Upload,
    Install(Principal),
    Readiness(Principal),
}

enum ChildEvidence {
    Unchanged,
    Pinned,
    Uploaded,
    Installed { hash: Option<Vec<u8>>, version: u64 },
    Ready,
}

async fn inspect_child(child: Principal) -> Result<mgmt::CanisterStatus, Failure> {
    let args = CanisterArgs { canister_id: child }
        .encode(1024)
        .expect("status args");
    let reply = management("canister_status", args, 0).await?;
    let value = mgmt::decode_canister_status(&reply, MANAGEMENT_LIMITS)
        .map_err(|_| Failure::InvalidReply)?;
    if value.settings.controllers != [api::canister_self()] {
        return Err(Failure::Mismatch);
    }
    Ok(value)
}

async fn observe_child(
    slot: ChildId,
    upgrade: bool,
    hash: [u8; 32],
    step: ChildObservation,
) -> Result<ChildEvidence, Failure> {
    match step {
        ChildObservation::Pin => {
            if delivery::source(slot, upgrade)
                .is_some_and(|source| source.state == stored::PinState::Issued)
            {
                delivery::pin(slot, upgrade).await?;
                Ok(ChildEvidence::Pinned)
            } else {
                Ok(ChildEvidence::Unchanged)
            }
        }
        ChildObservation::Upload => {
            delivery::observe_cache(slot, upgrade, hash).await?;
            Ok(ChildEvidence::Uploaded)
        }
        ChildObservation::Install(child) => {
            let value = inspect_child(child).await?;
            if value.module_hash.as_deref() != Some(hash.as_slice()) {
                return Err(Failure::Mismatch);
            }
            Ok(ChildEvidence::Installed {
                hash: value.module_hash,
                version: value.version,
            })
        }
        ChildObservation::Readiness(child) => {
            let readiness = STATE.with_borrow(|s| s.as_ref().expect("open Root").readiness.clone());
            let reply = Call::new(child, readiness.method, Wait::Bounded { seconds: 5 })
                .owned_args(readiness.request)
                .execute()
                .await
                .map_err(failure)
                .and_then(response)?;
            (readiness.accept)(&reply)?;
            Ok(ChildEvidence::Ready)
        }
    }
}

async fn observation(slot: ChildId) -> Result<(), Failure> {
    let before = snapshot(slot);
    let Some(a) = before.approval.as_ref() else {
        return Ok(());
    };
    let step = match before.phase {
        Phase::Scheduled => ChildObservation::Pin,
        Phase::UploadIssued => ChildObservation::Upload,
        Phase::InstallIssued | Phase::Installed => ChildObservation::Install(worker(&before)),
        Phase::Verified | Phase::Ready => ChildObservation::Readiness(worker(&before)),
        _ => return Ok(()),
    };
    match observe_child(slot, false, a.module_hash, step).await? {
        ChildEvidence::Unchanged => (),
        ChildEvidence::Pinned => mutate(slot, |s| s.failure = None),
        ChildEvidence::Uploaded => mutate(slot, |s| {
            s.phase = Phase::Uploaded;
            s.failure = None;
        }),
        ChildEvidence::Installed { hash, .. } => mutate(slot, |s| {
            s.phase = Phase::Verified;
            s.observed_module = hash;
            s.failure = None;
        }),
        ChildEvidence::Ready => {
            #[cfg(feature = "sharding")]
            sharding::bind_ready(slot, before.worker.as_deref().expect("verified worker"));
            mutate(slot, |s| {
                s.phase = Phase::Ready;
                s.failure = None;
            });
            if before.phase != Phase::Ready {
                directory::schedule(Some(slot));
            }
        }
    }
    Ok(())
}

/// Reconstruct observable outcomes without reissuing uncertain effects.
/// # Panics
/// Traps for noncontrollers, mismatched approvals, uninitialized storage or failed durable writes.
pub async fn reconcile(slot: ChildId, expected: canic_management_protocol::Approval) -> Status {
    assert!(api::is_controller(&api::caller()), "controller required");
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
pub(crate) fn reserved_cycles() -> u128 {
    STATE.with_borrow(|state| {
        u128::from(RESERVE_CYCLES)
            + state
                .as_ref()
                .expect("open engine")
                .journal
                .0
                .iter()
                .filter(|(_slot, journal)| {
                    #[cfg(feature = "sharding")]
                    if state
                        .as_ref()
                        .unwrap()
                        .pools
                        .standby_claim(**_slot)
                        .is_some()
                    {
                        return false;
                    }
                    journal.0.phase == Phase::Scheduled
                })
                .map(|_| u128::from(CREATION_CYCLES))
                .sum::<u128>()
    })
}

#[cfg(feature = "backup")]
pub(crate) fn snapshot_idle() -> bool {
    OBSERVING.with_borrow(BTreeSet::is_empty)
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
