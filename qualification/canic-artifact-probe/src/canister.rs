//! Isolated raw stable-memory and scheduled artifact fixture, not a production integration.
use ic_platform_types::Principal;

use crate::{
    ARTIFACT_LIMITS, AdmissionError, Approval, CHECKPOINT, Fault, LIMITS, Phase, RECONCILE,
    Retained, STAGE, STATUS, Status, UPGRADE, artifact_hash,
};
use ic_platform_protocol::{
    candid::Limits,
    management::{self as mgmt, CanisterArgs, InstallChunkedCodeArgs, InstallMode},
    management_chunks as chunks,
};
use ic_platform_runtime::{
    self as runtime,
    call::{Call, Outcome, Wait},
    raw as stable, system as api,
};
use std::cell::RefCell;

thread_local! {
    static TIMER: RefCell<Option<ic_timers::Once>> = const { RefCell::new(None) };
}
#[runtime::init(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn init((): ()) {
    open_timer();
}

fn open_timer() {
    ic_timers::initialize().expect("one artifact timer runtime");
    let timer = ic_timers::bind_once(
        ["qualification", "artifact", "install"],
        ic_timers::Lifetime::Retained,
        || async {
            install_artifact().await;
            ic_timers::OnceResult {
                observation: ic_timers::Observation::Success,
                next: ic_timers::OnceNext::Stop,
            }
        },
    )
    .expect("artifact timer");
    TIMER.with_borrow_mut(|slot| *slot = Some(timer));
    schedule();
}
fn schedule() {
    let desired = JOURNAL.with_borrow(|state| {
        state
            .approval
            .as_ref()
            .filter(|_| state.phase == Phase::Scheduled)
            .map_or(ic_timers::OnceDemand::Inactive, |a| {
                ic_timers::OnceDemand::At(a.execute_at_ns)
            })
    });
    TIMER.with_borrow_mut(|timer| {
        timer
            .as_mut()
            .expect("initialized timer")
            .set(desired)
            .expect("artifact demand")
    });
}

thread_local! {
    static JOURNAL: RefCell<Retained> = RefCell::new(Retained::default());
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

fn guard(limit: usize) -> Result<(), String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    if ic_platform_runtime::system::argument_size() > limit {
        return Err("request too large".into());
    }
    Ok(())
}

fn controller() -> Result<(), String> {
    guard(LIMITS.bytes)
}

fn uploader() -> Result<(), String> {
    guard(ARTIFACT_LIMITS.bytes)
}

fn decode_stage(bytes: &[u8]) -> Result<Approval, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Request,
        },
        ARTIFACT_LIMITS,
    )
}

#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns its reply")]
fn encode_stage(value: Result<Status, AdmissionError>) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::update(runtime = runtime, guard = uploader, decode = decode_stage, max_bytes = 2097152, on_error = endpoint_error, name = "stage", encode = encode_stage)]
fn stage(approval: Approval) -> Result<Status, AdmissionError> {
    let previous = JOURNAL.with_borrow(|state| state.approval.clone());
    if let Some(previous) = previous {
        return if previous == approval {
            Ok(status(()))
        } else {
            Err(AdmissionError::Occupied)
        };
    }
    approval.validate(api::time())?;
    if approval.target == api::canister_self().as_slice() {
        return Err(AdmissionError::Target);
    }
    JOURNAL.with_borrow_mut(|state| {
        state.approval = Some(approval);
        state.phase = Phase::Scheduled;
    });
    schedule();
    Ok(status(()))
}

fn decode_status(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns its reply")]
fn encode_status(value: Status) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::query(runtime = runtime, guard = controller, decode = decode_status, max_bytes = 2097152, on_error = endpoint_error, name = "status", encode = encode_status)]
fn status((): ()) -> Status {
    JOURNAL.with_borrow(|state| Status {
        phase: state.phase,
        attempts: state.attempts,
        upload_attempts: state.upload_attempts,
        upload_request_sha256: state.upload_request_sha256,
        chunk_hash: state.chunk_hash.clone(),
        reply_retained: state.reply_retained,
        request_sha256: state.request_sha256,
        target: state.approval.as_ref().map(|a| a.target.clone()),
        sha256: state.approval.as_ref().map(|a| a.sha256),
        wasm_bytes: state.approval.as_ref().map_or(0, |a| a.wasm.len()),
        observed_module: state.observed_module.clone(),
        maintenance_candidate: cfg!(feature = "maintenance-candidate"),
        stable_pages: ic_platform_runtime::raw::stable64_size(),
    })
}

// Heartbeat is deliberately confined to this isolated experiment. It supplies
// operator-independent execution without qualifying a production timer policy.
async fn install_artifact() {
    let approval = JOURNAL.with_borrow(|state| {
        state
            .approval
            .as_ref()
            .filter(|a| state.phase == Phase::Scheduled && api::time() >= a.execute_at_ns)
            .cloned()
    });
    let Some(approval) = approval else { return };
    let upload =
        chunks::encode_upload_chunk(api::canister_self(), &approval.wasm, ARTIFACT_LIMITS.bytes)
            .expect("bounded management upload request");
    JOURNAL.with_borrow_mut(|state| {
        state.phase = Phase::UploadIssued;
        state.upload_attempts = 1;
        state.upload_request_sha256 = Some(artifact_hash(&upload));
    });
    let reply = Call::new(
        ic_platform_types::Principal::management_canister(),
        "upload_chunk",
        Wait::Unbounded,
    )
    .owned_args(upload)
    .execute()
    .await
    .expect("chunk upload reply");
    let Outcome::Reply(reply) = reply.into_parts().0 else {
        api::trap("chunk upload refused")
    };
    let chunk = chunks::decode_upload_chunk(&reply, MANAGEMENT_LIMITS).expect("chunk hash");
    assert_eq!(chunk.hash, approval.sha256, "uploaded chunk hash mismatch");
    JOURNAL.with_borrow_mut(|state| state.chunk_hash = Some(chunk.hash.clone()));
    if approval.fault == Fault::AfterUploadWrite {
        api::trap("injected upload callback trap after result write");
    }
    let request = InstallChunkedCodeArgs {
        mode: InstallMode::Install,
        target_canister: Principal::from_slice(&approval.target),
        store_canister: Some(api::canister_self()),
        chunk_hashes_list: vec![chunk],
        wasm_module_hash: approval.sha256.to_vec(),
        arg: vec![],
        sender_canister_version: None,
    }
    .encode(ARTIFACT_LIMITS.bytes)
    .expect("bounded management install request");
    JOURNAL.with_borrow_mut(|state| {
        state.phase = Phase::Issued;
        state.attempts = 1;
        state.request_sha256 = Some(artifact_hash(&request));
    });
    let result = Call::new(
        ic_platform_types::Principal::management_canister(),
        "install_chunked_code",
        Wait::Unbounded,
    )
    .owned_args(request)
    .execute()
    .await;
    match result.map(|completion| completion.into_parts().0) {
        Ok(Outcome::Reply(reply)) => {
            mgmt::decode_empty_response(&reply, MANAGEMENT_LIMITS).expect("install success reply");
            JOURNAL.with_borrow_mut(|state| {
                state.reply_retained = true;
                state.phase = Phase::Installed;
            });
            if approval.fault == Fault::AfterResultWrite {
                api::trap("injected install callback trap after result write");
            }
            // This management call commits the install reply before verification.
            verify_target().await;
        }
        Ok(Outcome::Rejected(rejection)) => JOURNAL.with_borrow_mut(|state| {
            state.phase = Phase::Rejected(rejection.code());
        }),
        // Keep the slot blocked on local dispatch errors too; this fixture makes
        // no recovery claim for that separate boundary and never retries install.
        Err(_) | Ok(Outcome::OutcomeUnknown(_)) => (),
    }
}

async fn verify_target() {
    let (target, expected) = JOURNAL.with_borrow(|state| {
        assert!(
            matches!(
                state.phase,
                Phase::Issued | Phase::Installed | Phase::Verified
            ),
            "no issued installation"
        );
        let approval = state.approval.as_ref().expect("retained approval");
        (Principal::from_slice(&approval.target), approval.sha256)
    });
    let request = CanisterArgs {
        canister_id: target,
    }
    .encode(1024)
    .expect("status request");
    let reply = Call::new(
        ic_platform_types::Principal::management_canister(),
        "canister_status",
        Wait::Unbounded,
    )
    .owned_args(request)
    .execute()
    .await
    .expect("management observation");
    let Outcome::Reply(reply) = reply.into_parts().0 else {
        api::trap("management observation refused")
    };
    let observed =
        mgmt::decode_canister_status(&reply, MANAGEMENT_LIMITS).expect("bounded management status");
    assert_eq!(
        observed.module_hash.as_deref(),
        Some(expected.as_slice()),
        "installed hash mismatch"
    );
    assert_eq!(
        observed.settings.controllers,
        vec![api::canister_self()],
        "sole executor controller required"
    );
    JOURNAL.with_borrow_mut(|state| {
        state.observed_module = observed.module_hash;
        state.phase = Phase::Verified;
    });
}

fn decode_reconcile(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: RECONCILE,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns its reply")]
fn encode_reconcile(value: Status) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: RECONCILE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::update(runtime = runtime, guard = controller, decode = decode_reconcile, max_bytes = 2097152, on_error = endpoint_error, name = "reconcile", encode = encode_reconcile)]
async fn reconcile((): ()) -> Status {
    verify_target().await;
    status(())
}

#[runtime::pre_upgrade(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn pre_upgrade((): ()) {
    let bytes = JOURNAL
        .with_borrow(|state| {
            ic_wire::encode(
                state,
                ic_wire::Encoding::Frame {
                    method: CHECKPOINT,
                    direction: ic_wire::Direction::Response,
                },
                ARTIFACT_LIMITS,
            )
        })
        .expect("bounded artifact checkpoint");
    let pages = (bytes.len() as u64 + 4).div_ceil(65_536);
    let existing = ic_platform_runtime::raw::stable64_size();
    if pages > existing {
        stable::stable64_grow(pages - existing).expect("checkpoint allocation");
    }
    stable::stable64_write(
        0,
        &u32::try_from(bytes.len())
            .expect("bounded length")
            .to_le_bytes(),
    );
    stable::stable64_write(4, &bytes);
}

fn decode_upgrade(bytes: &[u8]) -> Result<bool, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: UPGRADE,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

#[runtime::post_upgrade(runtime = runtime, guard = lifecycle_guard, decode = decode_upgrade, max_bytes = 2097152, on_error = endpoint_error)]
fn post_upgrade(trap_after_restore: bool) {
    assert!(
        ic_platform_runtime::raw::stable64_size() > 0,
        "artifact checkpoint missing"
    );
    let mut length = [0; 4];
    stable::stable64_read(0, &mut length);
    let length = u32::from_le_bytes(length) as usize;
    assert!(
        length > 0 && length <= ARTIFACT_LIMITS.bytes,
        "checkpoint length outside bound"
    );
    let mut bytes = vec![0; length];
    stable::stable64_read(4, &mut bytes);
    let state: Retained = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: CHECKPOINT,
            direction: ic_wire::Direction::Response,
        },
        ARTIFACT_LIMITS,
    )
    .expect("valid artifact checkpoint");
    if let Some(approval) = &state.approval {
        approval
            .validate_contents()
            .expect("valid retained artifact");
    }
    JOURNAL.with_borrow_mut(|journal| *journal = state);
    open_timer();
    if trap_after_restore {
        api::trap("injected artifact maintenance trap");
    }
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
