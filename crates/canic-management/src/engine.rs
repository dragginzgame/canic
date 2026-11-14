use crate::Readiness;
use canic_management_protocol::{
    CREATION_CYCLES, Failure, LIMITS, MAX_WASM, Phase, RESERVE_CYCLES, Refusal, STAGE_LIMITS,
    Stage, Status, digest,
};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{
        CanisterIdRecord, CanisterInstallMode, CanisterSettings, CanisterStatusResult, ChunkHash,
        CreateCanisterArgs, InstallChunkedCodeArgs, UploadChunkArgs,
    },
};
use ic_platform_runtime::{
    Principal,
    call::{Call, CallFailed, WaitPolicy},
    cdk::api,
};
use ic_storage::{
    AllocationGrants, DefaultMemoryImpl, GenericAllocationPolicy, MemoryAuthority, MemoryRequest,
    MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::{Bound, CodecError, StableCell, Storable},
};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, TimerCompletion, TimerIdentity, TimerSchedule,
};
use std::{borrow::Cow, cell::RefCell};

const MAX_PAGES: u64 = 64;
const MANAGEMENT_LIMITS: decoder::Limits = decoder::Limits {
    bytes: 16_384,
    header: 4096,
    types: 256,
    work: 65_536,
    allocation: 65_536,
};

struct Journal(Status);
impl Storable for Journal {
    const BOUND: Bound = Bound::Bounded {
        max_size: 4096,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        ic_wire::native::encode(&self.0, LIMITS)
            .map(Cow::Owned)
            .map_err(|_| CodecError::Invalid("journal"))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        ic_wire::native::decode(&bytes, LIMITS)
            .map(Self)
            .map_err(|_| CodecError::Invalid("journal"))
    }
}
struct Artifact(Vec<u8>);
impl Storable for Artifact {
    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_WASM as u32,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        Ok(Cow::Borrowed(&self.0))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        Ok(Self(bytes.into_owned()))
    }
}
struct State {
    journal: StableCell<Journal>,
    artifact: StableCell<Artifact>,
    readiness: Readiness,
    _runtime: MemoryRuntime<DefaultMemoryImpl>,
}
thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
    static OBSERVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    #[cfg(feature = "fault-injection")]
    static FAULT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Open the one-child engine and register its continuation timer.
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
    if !fresh {
        assert_eq!(
            runtime.diagnostics().expect("integrity").allocations,
            2,
            "missing manager stores"
        );
    }
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
        StableCell::create(journal, Journal(Status::default()))
    } else {
        StableCell::load(journal, 4096)
    }
    .expect("strict journal opening");
    let artifact = if fresh {
        StableCell::create(artifact, Artifact(vec![]))
    } else {
        StableCell::load(artifact, MAX_WASM)
    }
    .expect("strict artifact opening");
    let state = State {
        journal,
        artifact,
        readiness,
        _runtime: runtime,
    };
    validate(&state);
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    ic_timers::initialize_runtime().expect("one timer runtime");
    schedule();
}

fn validate(state: &State) {
    let s = &state.journal.get().0;
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
            digest(&state.artifact.get().0),
            a.module_hash,
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
        assert!(state.artifact.get().0.is_empty(), "unadmitted artifact");
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

fn snapshot() -> Status {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("initialized manager")
            .journal
            .get()
            .0
            .clone()
    })
}
fn mutate(f: impl FnOnce(&mut Status)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut journal = state.journal.get().0.clone();
        f(&mut journal);
        state.journal.set(Journal(journal));
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
fn schedule() {
    let desired = next(&snapshot()).map(TimerSchedule::At);
    TIMER.with_borrow_mut(|slot| {
        ic_timers::reconcile_once(
            slot,
            &TimerIdentity::try_new("profile", "manager", "one-worker").expect("constant timer"),
            desired,
            |_| async {
                if OBSERVING.replace(true) {
                    return OnceRunResult::new(TimerCompletion::no_work(), OnceDecision::Stop);
                }
                step().await;
                OBSERVING.set(false);
                let decision =
                    next(&snapshot()).map_or(OnceDecision::Stop, OnceDecision::ScheduleAt);
                OnceRunResult::new(TimerCompletion::success(1), decision)
            },
        )
        .expect("schedule retained demand")
    });
}
/// Read committed evidence without resuming work.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn status() -> Status {
    let mut value = snapshot();
    value.stable_pages = api::stable_size();
    value.maintenance_candidate = cfg!(feature = "maintenance-candidate");
    value
}
/// Retain one exact controller-approved artifact before scheduling effects.
/// # Errors
/// Refuses changed authority, invalid artifact/binding/time or inadequate reserve.
/// # Panics
/// Traps for noncontrollers, uninitialized storage or failed durable writes.
pub fn stage(input: Stage) -> Result<Status, Refusal> {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    if let Some(previous) = snapshot().approval {
        return if previous == input.approval && digest(&input.wasm) == previous.module_hash {
            Ok(status())
        } else {
            Err(Refusal::Occupied)
        };
    }
    input.validate(&api::canister_self().to_text(), api::time())?;
    if api::canister_liquid_cycle_balance() < u128::from(CREATION_CYCLES + RESERVE_CYCLES) {
        return Err(Refusal::Reserve);
    }
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("initialized manager")
            .artifact
            .set(Artifact(input.wasm));
    });
    mutate(|s| {
        s.approval = Some(input.approval);
        s.phase = Phase::Scheduled;
    });
    schedule();
    Ok(status())
}

fn fault(point: u8) {
    #[cfg(feature = "fault-injection")]
    if FAULT.get() == point {
        api::trap("injected growth boundary");
    }
    #[cfg(not(feature = "fault-injection"))]
    let _ = point;
}
#[cfg(feature = "fault-injection")]
/// Select a test-only interrupted-message boundary.
/// # Panics
/// Traps unless the caller controls this canister and the point is in range.
pub fn inject_fault(value: u8) {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    assert!(value <= 5);
    FAULT.set(value);
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
fn issue(index: usize, phase: Phase, bytes: &[u8]) {
    mutate(|s| {
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

async fn step() {
    let s = snapshot();
    if matches!(s.phase, Phase::Scheduled | Phase::Created | Phase::Uploaded) {
        let a = s.approval.as_ref().expect("approval");
        if api::time() > a.expires_at_ns {
            mutate(|s| s.phase = Phase::Expired);
            return;
        }
        if api::time() < a.execute_at_ns {
            return;
        }
        let required = RESERVE_CYCLES
            + if s.phase == Phase::Scheduled {
                CREATION_CYCLES
            } else {
                0
            };
        if api::canister_liquid_cycle_balance() < u128::from(required) {
            mutate(|s| s.phase = Phase::ReserveBlocked);
            return;
        }
    }
    match s.phase {
        Phase::Scheduled => {
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
            issue(0, Phase::CreateIssued, &bytes);
            fault(1); // Trap rolls back intent and prevents dispatch; original demand survives.
            let reply = management("create_canister", bytes, u128::from(CREATION_CYCLES)).await;
            let refunded = api::msg_cycles_refunded();
            match reply.and_then(|bytes| {
                decoder::decode_one::<CanisterIdRecord>(&bytes, MANAGEMENT_LIMITS)
                    .map_err(|_| Failure::InvalidReply)
            }) {
                Ok(value) => mutate(|s| {
                    s.worker = Some(value.canister_id.to_text());
                    s.creation_refund = Some(refunded);
                    s.phase = Phase::Created;
                }),
                Err(error) => mutate(|s| {
                    s.failure = Some(error);
                    s.creation_refund = Some(refunded);
                }),
            }
            fault(2); // Creation-result loss must remain blocked, including across upgrade.
        }
        Phase::Created => {
            fault(3); // A later message cannot erase the already committed principal.
            let wasm =
                STATE.with_borrow(|state| state.as_ref().expect("state").artifact.get().0.clone());
            let bytes = candid_codec::encode_one(
                &UploadChunkArgs {
                    canister_id: api::canister_self(),
                    chunk: wasm,
                },
                STAGE_LIMITS.bytes,
            )
            .expect("upload args");
            issue(1, Phase::UploadIssued, &bytes);
            let result = management("upload_chunk", bytes, 0)
                .await
                .and_then(|bytes| {
                    decoder::decode_one::<ChunkHash>(&bytes, MANAGEMENT_LIMITS)
                        .map_err(|_| Failure::InvalidReply)
                });
            match result {
                Ok(hash) if hash.hash == s.approval.expect("approval").module_hash => {
                    mutate(|s| s.phase = Phase::Uploaded)
                }
                Ok(_) => mutate(|s| s.failure = Some(Failure::Mismatch)),
                Err(error) => mutate(|s| s.failure = Some(error)),
            }
            fault(4);
        }
        Phase::Uploaded => {
            let hash = s.approval.as_ref().expect("approval").module_hash.to_vec();
            let bytes = candid_codec::encode_one(
                &InstallChunkedCodeArgs {
                    mode: CanisterInstallMode::Install,
                    target_canister: worker(&s),
                    store_canister: Some(api::canister_self()),
                    chunk_hashes_list: vec![ChunkHash { hash: hash.clone() }],
                    wasm_module_hash: hash,
                    arg: vec![],
                    sender_canister_version: Some(api::canister_version()),
                },
                4096,
            )
            .expect("install args");
            issue(2, Phase::InstallIssued, &bytes);
            let result = management("install_chunked_code", bytes, 0)
                .await
                .and_then(|bytes| {
                    decoder::decode_args::<()>(&bytes, MANAGEMENT_LIMITS, decoder::Arity::Exact)
                        .map_err(|_| Failure::InvalidReply)
                });
            match result {
                Ok(()) => mutate(|s| s.phase = Phase::Installed),
                Err(error) => mutate(|s| s.failure = Some(error)),
            }
            fault(5);
        }
        Phase::Installed | Phase::Verified => {
            if let Err(error) = observation().await {
                mutate(|s| s.failure = Some(error));
            }
        }
        _ => {}
    }
}

async fn observation() -> Result<(), Failure> {
    let before = snapshot();
    let Some(a) = before.approval.as_ref() else {
        return Ok(());
    };
    match before.phase {
        Phase::UploadIssued => {
            let bytes = candid_codec::encode_one(
                &CanisterIdRecord {
                    canister_id: api::canister_self(),
                },
                1024,
            )
            .expect("chunks args");
            let reply = management("stored_chunks", bytes, 0).await?;
            let chunks: Vec<ChunkHash> = decoder::decode_one(&reply, MANAGEMENT_LIMITS)
                .map_err(|_| Failure::InvalidReply)?;
            if !chunks.iter().any(|chunk| chunk.hash == a.module_hash) {
                return Err(Failure::Mismatch);
            }
            mutate(|s| {
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
            mutate(|s| {
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
            mutate(|s| {
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
/// Traps for noncontrollers, uninitialized storage or failed durable writes.
pub async fn reconcile() -> Status {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    if OBSERVING.replace(true) {
        return status();
    }
    if let Err(error) = observation().await {
        mutate(|s| s.failure = Some(error));
    }
    OBSERVING.set(false);
    schedule();
    status()
}
