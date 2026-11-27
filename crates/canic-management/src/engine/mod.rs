pub(crate) mod application;
#[cfg(feature = "blob")]
pub(crate) mod blob;
mod delivery;
mod scheduler;
use scheduler::{resume_after_reconciliation, schedule};
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
    storage::{ARTIFACT_KEY, Artifact, ChildRows, JOURNAL_KEY, Journal, KEYS, META_KEY, Metadata},
};
use canic_management_protocol::{
    Approval, CREATION_CYCLES, Failure, Phase, RESERVE_CYCLES, Refusal, Status,
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
#[cfg(feature = "fault-injection")]
use std::collections::BTreeMap;
use std::{cell::RefCell, collections::BTreeSet};

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
            .rows()
            .keys()
            .copied()
            .filter(|id| id.service_index().is_some())
            .collect()
    })
}

pub(crate) fn service_for_worker(worker: &str) -> Option<ChildId> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .expect("open engine")
            .journal
            .rows()
            .iter()
            .find_map(|(&slot, journal)| {
                (slot.service_index().is_some() && journal.0.worker.as_deref() == Some(worker))
                    .then_some(slot)
            })
    })
}

/// Retain the fleet ceiling during a fresh Root installation.
/// # Panics
/// Traps if the engine is closed, the policy is invalid or bootstrap already ran.
pub fn bootstrap(policy: canic_management_protocol::stored::RootBootstrap) {
    assert!(policy.valid(), "invalid Root bootstrap policy");
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("initialized Root");

        assert!(
            state.metadata.bootstrap.replace(policy).is_none(),
            "already bootstrapped"
        );
        state.save_metadata();
    });
}

impl crate::storage::Journal {
    fn custody(&self) -> canic_management_protocol::custody::Custody {
        let mut upgrades = self.2.clone();
        if self.1.approval.is_some() {
            upgrades.push(self.1.clone());
        }
        canic_management_protocol::custody::Custody {
            creation: self.0.clone(),
            upgrades,
        }
    }
}

fn journals() -> children::Custody {
    STATE.with_borrow(|state| {
        let journals = &state.as_ref().expect("open engine").journal.rows();
        service_slots()
            .into_iter()
            .map(|slot| {
                let journal = &journals[&slot];
                journal.custody()
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
    open_version: u64,
    journal: ChildRows<Journal>,
    admitted: usize,
    scheduled: usize,
    artifact: ChildRows<Artifact>,
    metadata: Metadata,
    metadata_rows: ChildRows<Metadata>,
    #[cfg(feature = "sharding")]
    pool: sharding::PlacementRows,
    #[cfg(feature = "sharding")]
    pools: sharding::Pools,
    #[cfg(feature = "sharding")]
    worker_history: ic_storage::adapter::StableBTreeMap<[u8; 32], Vec<u8>>,
    #[cfg(feature = "role-issuer")]
    role_key: StableCell<Vec<u8>>,
    #[cfg(feature = "auth")]
    auth_history: StableCell<Vec<u8>>,
    readiness: Readiness,
    pub(crate) backup: crate::backup::Ledger,
}
/// Whether this Root has issued a receiver grant since fresh installation.
/// # Panics
/// Requires an opened management state.
#[cfg(feature = "auth")]
#[must_use]
pub fn auth_grant_issued() -> bool {
    STATE.with_borrow(|s| s.as_ref().expect("open manager").auth_history.get() == &[1])
}

/// Record the first successful grant in the same message as admission.
/// # Panics
/// Requires an opened management state.
#[cfg(feature = "auth")]
pub fn record_auth_grant() {
    STATE.with_borrow_mut(|s| {
        let history = &mut s.as_mut().expect("open manager").auth_history;
        if history.get() == &[0] {
            history.set(vec![1]);
        }
    });
}

impl State {
    fn set_journal(&mut self, slot: ChildId, journal: Journal) {
        let previous = self.journal.rows().get(&slot);
        self.admitted -= usize::from(previous.is_some_and(|j| j.0.approval.is_some()));
        self.scheduled -= usize::from(previous.is_some_and(|j| j.0.phase == Phase::Scheduled));
        self.admitted += usize::from(journal.0.approval.is_some());
        self.scheduled += usize::from(journal.0.phase == Phase::Scheduled);
        self.journal.set(slot, journal).expect("child journal");
    }

    fn service_count(&self) -> usize {
        self.journal
            .rows()
            .last_key_value()
            .and_then(|(id, _)| id.service_index())
            .map_or(0, |index| index + 1)
    }

    fn save_metadata(&mut self) {
        self.metadata_rows
            .set(ChildId::FIRST, self.metadata.clone())
            .expect("management metadata");
    }
}

thread_local! {
    pub(crate) static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static OBSERVING: RefCell<BTreeSet<ChildId>> = const { RefCell::new(BTreeSet::new()) };
    #[cfg(feature = "fault-injection")]
    static FAULT: RefCell<BTreeMap<ChildId, u8>> = const { RefCell::new(BTreeMap::new()) };
}

/// Open the child engine and its shared scheduler.
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
    let journal = ChildRows::<Journal>::open(&runtime, JOURNAL_KEY, fresh);
    let artifact = ChildRows::open(&runtime, ARTIFACT_KEY, fresh);
    let mut metadata_rows = ChildRows::<Metadata>::open(&runtime, META_KEY, fresh);
    if fresh {
        metadata_rows
            .set(ChildId::FIRST, Metadata::default())
            .expect("management metadata");
    }
    let metadata = metadata_rows.rows()[&ChildId::FIRST].clone();
    let backup = crate::backup::Ledger::open(&runtime, fresh);
    #[cfg(feature = "sharding")]
    let (pool, pools) = sharding::PlacementRows::open(&runtime, fresh);
    #[cfg(feature = "sharding")]
    let worker_history = canic::storage::map(&runtime, crate::storage::WORKER_HISTORY_KEY, fresh);
    #[cfg(feature = "auth")]
    let auth_history = canic::storage::cell(
        &runtime,
        crate::storage::AUTH_HISTORY_KEY,
        1,
        fresh.then(|| vec![0]),
    );
    #[cfg(feature = "auth")]
    assert!(
        matches!(auth_history.get().as_slice(), [0] | [1]),
        "invalid grant history"
    );
    let state = State {
        open_version: api::canister_version(),
        #[cfg(feature = "auth")]
        auth_history,
        admitted: journal
            .rows()
            .values()
            .filter(|j| j.0.approval.is_some())
            .count(),
        scheduled: journal
            .rows()
            .values()
            .filter(|j| j.0.phase == Phase::Scheduled)
            .count(),
        journal,
        artifact,
        metadata,
        metadata_rows,
        #[cfg(feature = "role-issuer")]
        role_key: role_issuer::open(&runtime, fresh),
        #[cfg(feature = "sharding")]
        pools,

        #[cfg(feature = "sharding")]
        worker_history,
        #[cfg(feature = "sharding")]
        pool,
        backup,
        readiness,
    };
    assert!(
        state.journal.rows().keys().eq(state.artifact.rows().keys()),
        "matching child custody"
    );
    let slots: Vec<_> = state.journal.rows().keys().copied().collect();
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
    store_maintenance::validate(&state);
    let mut children = BTreeSet::new();
    for (slot, journal) in state.journal.rows() {
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
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    canic::lifecycle::initialize_diagnostics();
    canic::lifecycle::initialize_timers().expect("one timer runtime");
    scheduler::open(slots);
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
    let s = &state.journal.rows()[&slot].0;
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
            state.artifact.rows()[&slot].hash(false),
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
            state.artifact.rows()[&slot].hash(false).is_none(),
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
            .rows()
            .get(&slot)
            .map(|j| j.0.clone())
            .unwrap_or_default()
    })
}

fn mutate(slot: ChildId, f: impl FnOnce(&mut Status)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut journal = state.journal.rows().get(&slot).cloned().unwrap_or_default();
        let previous_phase = journal.0.phase;
        let previous_failure = journal.0.failure.clone();
        let next = &mut journal.0;
        f(next);
        if previous_phase != next.phase || previous_failure != next.failure {
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
        state.set_journal(slot, journal);
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

/// Read committed evidence without resuming work.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn status(slot: ChildId) -> Status {
    let mut value = snapshot(slot);
    value.stable_pages = ic_platform_runtime::raw::stable64_size();
    value
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
            .is_none_or(|index| index > STATE.with_borrow(|s| s.as_ref().unwrap().service_count()))
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
            let maximum = state.metadata.bootstrap.map_or(
                canic_management_protocol::stored::DEFAULT_SUBNET_CHILD_LIMIT,
                |policy| policy.max_children,
            );
            state.admitted >= maximum as usize
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
        .ok_or(Refusal::Artifact)?
        .creation
        .operation;
    let liquid = ic_platform_runtime::raw::liquid_cycle_balance();
    #[cfg(feature = "sharding")]
    let liquid = match origin {
        Origin::Paid(_, credit) => liquid.saturating_add(credit),
        _ => liquid,
    };
    if liquid < reserved_cycles() + attachment {
        return Err(Refusal::Reserve);
    }
    STATE
        .with_borrow_mut(|state| {
            state
                .as_mut()
                .expect("open engine")
                .artifact
                .set(slot, artifact)
        })
        .map_err(|_| Refusal::Artifact)?;
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
            canic_management_protocol::application::STAGE_CHILD_CREATION
        ) | (
            Some(10),
            canic_management_protocol::application::RECONCILE_CHILD_CREATION
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

/// Set native download rights for an exact ready permanent child.
///
/// # Errors
/// Refuses invalid readers, a foreign or unready target, or a failed management call.
pub async fn configure_snapshot_readers(input: stored::SnapshotReaders) -> Result<(), Failure> {
    if !api::is_controller(&api::caller())
        || input.root != api::canister_self()
        || !stored::valid_snapshot_readers(&input.readers)
    {
        return Err(Failure::Mismatch);
    }
    let target = input.child.to_text();
    let slot = STATE
        .with_borrow(|state| {
            state
                .as_ref()?
                .journal
                .rows()
                .iter()
                .find_map(|(&slot, j)| {
                    (slot.service_index().is_some()
                        && j.0.phase == Phase::Ready
                        && j.0.worker.as_ref() == Some(&target))
                    .then_some(slot)
                })
        })
        .ok_or(Failure::Mismatch)?;
    if crate::backup::target_occupied(&target) {
        return Err(Failure::Uncertain);
    }
    let _guard = observing(slot).ok_or(Failure::Uncertain)?;
    let visibility = if input.readers.is_empty() {
        mgmt::SnapshotVisibility::Controllers
    } else {
        mgmt::SnapshotVisibility::AllowedViewers(input.readers)
    };
    let args = mgmt::UpdateSettingsArgs {
        canister_id: input.child,
        settings: CanisterSettings {
            snapshot_visibility: Some(visibility),
            ..Default::default()
        },
        sender_canister_version: Some(api::canister_version()),
    }
    .encode(4096)
    .map_err(|_| Failure::Mismatch)?;
    let reply = management("update_settings", args, 0).await?;
    mgmt::decode_empty_response(&reply, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)
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
        let state = state.as_ref().expect("open engine");
        let count = state.scheduled;
        #[cfg(feature = "sharding")]
        let count = count - state.pools.scheduled_spares(state.journal.rows());
        u128::from(RESERVE_CYCLES) + count as u128 * u128::from(CREATION_CYCLES)
    })
}

pub(crate) fn snapshot_idle(slot: ChildId) -> bool {
    OBSERVING.with_borrow(|active| !active.contains(&slot))
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
