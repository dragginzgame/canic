//! Isolated raw stable-memory and heartbeat fixture, not a production integration.

use crate::{
    ARTIFACT_LIMITS, AdmissionError, Approval, CHECKPOINT, Fault, LIMITS, Phase, RECONCILE,
    Retained, STAGE, STATUS, Status, UPGRADE, artifact_hash,
};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{
        CanisterIdRecord, CanisterInstallMode, CanisterStatusResult, ChunkHash,
        InstallChunkedCodeArgs, UploadChunkArgs,
    },
};
use ic_platform_runtime::{
    Principal,
    call::{Call, CallFailed, WaitPolicy},
    cdk::{self, api, stable},
};
use std::cell::RefCell;

thread_local! {
    static JOURNAL: RefCell<Retained> = RefCell::new(Retained::default());
}

const MANAGEMENT_LIMITS: decoder::Limits = decoder::Limits {
    bytes: 16_384,
    header: 4096,
    types: 256,
    work: 65_536,
    allocation: 65_536,
};

fn guard(limit: usize) -> Result<(), String> {
    if !api::is_controller(&api::msg_caller()) {
        return Err("controller required".into());
    }
    if ic_platform_runtime::ic0::msg_arg_data_size() > limit {
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

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns argument bytes"
)]
fn decode_stage(bytes: Vec<u8>) -> Approval {
    ic_wire::native::decode_request(STAGE, &bytes, ARTIFACT_LIMITS).expect("bounded approval")
}

#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns its reply")]
fn encode_stage(value: Result<Status, AdmissionError>) -> Vec<u8> {
    ic_wire::native::encode_response(STAGE, &value, LIMITS).expect("bounded staging result")
}

#[cdk::update(
    guard = "uploader",
    decode_with = "decode_stage",
    encode_with = "encode_stage"
)]
fn stage(approval: Approval) -> Result<Status, AdmissionError> {
    let previous = JOURNAL.with_borrow(|state| state.approval.clone());
    if let Some(previous) = previous {
        return if previous == approval {
            Ok(status())
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
    Ok(status())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns argument bytes"
)]
fn decode_status(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(STATUS, &bytes, LIMITS).expect("status request");
}

#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns its reply")]
fn encode_status(value: Status) -> Vec<u8> {
    ic_wire::native::encode_response(STATUS, &value, LIMITS).expect("bounded status")
}

#[cdk::query(
    guard = "controller",
    decode_with = "decode_status",
    encode_with = "encode_status"
)]
fn status() -> Status {
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
        stable_pages: api::stable_size(),
    })
}

// Heartbeat is deliberately confined to this isolated experiment. It supplies
// operator-independent execution without qualifying a production timer policy.
#[cdk::heartbeat]
async fn heartbeat() {
    let approval = JOURNAL.with_borrow(|state| {
        state
            .approval
            .as_ref()
            .filter(|a| state.phase == Phase::Scheduled && api::time() >= a.execute_at_ns)
            .cloned()
    });
    let Some(approval) = approval else { return };
    let upload = candid_codec::encode_one(
        &UploadChunkArgs {
            canister_id: api::canister_self(),
            chunk: approval.wasm,
        },
        ARTIFACT_LIMITS.bytes,
    )
    .expect("bounded management upload request");
    JOURNAL.with_borrow_mut(|state| {
        state.phase = Phase::UploadIssued;
        state.upload_attempts = 1;
        state.upload_request_sha256 = Some(artifact_hash(&upload));
    });
    let reply = Call::new(
        Principal::management_canister(),
        "upload_chunk",
        WaitPolicy::Unbounded,
    )
    .take_raw_args(upload)
    .execute()
    .await
    .expect("chunk upload reply");
    let chunk: ChunkHash = decoder::decode_one(&reply, MANAGEMENT_LIMITS).expect("chunk hash");
    assert_eq!(chunk.hash, approval.sha256, "uploaded chunk hash mismatch");
    JOURNAL.with_borrow_mut(|state| state.chunk_hash = Some(chunk.hash.clone()));
    if approval.fault == Fault::AfterUploadWrite {
        api::trap("injected upload callback trap after result write");
    }
    let request = candid_codec::encode_one(
        &InstallChunkedCodeArgs {
            mode: CanisterInstallMode::Install,
            target_canister: Principal::from_slice(&approval.target),
            store_canister: Some(api::canister_self()),
            chunk_hashes_list: vec![chunk],
            wasm_module_hash: approval.sha256.to_vec(),
            arg: vec![],
            sender_canister_version: None,
        },
        ARTIFACT_LIMITS.bytes,
    )
    .expect("bounded management install request");
    JOURNAL.with_borrow_mut(|state| {
        state.phase = Phase::Issued;
        state.attempts = 1;
        state.request_sha256 = Some(artifact_hash(&request));
    });
    let result = Call::new(
        Principal::management_canister(),
        "install_chunked_code",
        WaitPolicy::Unbounded,
    )
    .take_raw_args(request)
    .execute()
    .await;
    match result {
        Ok(reply) => {
            decoder::decode_args::<()>(&reply, MANAGEMENT_LIMITS, decoder::Arity::Exact)
                .expect("install success reply");
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
        Err(CallFailed::CallRejected(rejection)) => JOURNAL.with_borrow_mut(|state| {
            state.phase = Phase::Rejected(rejection.raw_reject_code());
        }),
        // Keep the slot blocked on local dispatch errors too; this fixture makes
        // no recovery claim for that separate boundary and never retries install.
        Err(_) => (),
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
    let request = candid_codec::encode_one(
        &CanisterIdRecord {
            canister_id: target,
        },
        1024,
    )
    .expect("status request");
    let reply = Call::new(
        Principal::management_canister(),
        "canister_status",
        WaitPolicy::Unbounded,
    )
    .take_raw_args(request)
    .execute()
    .await
    .expect("management observation");
    let observed: CanisterStatusResult =
        decoder::decode_one(&reply, MANAGEMENT_LIMITS).expect("bounded management status");
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

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns argument bytes"
)]
fn decode_reconcile(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(RECONCILE, &bytes, LIMITS).expect("reconcile request");
}

#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns its reply")]
fn encode_reconcile(value: Status) -> Vec<u8> {
    ic_wire::native::encode_response(RECONCILE, &value, LIMITS).expect("bounded reconciliation")
}

#[cdk::update(
    guard = "controller",
    decode_with = "decode_reconcile",
    encode_with = "encode_reconcile"
)]
async fn reconcile() -> Status {
    verify_target().await;
    status()
}

#[cdk::pre_upgrade]
fn pre_upgrade() {
    let bytes = JOURNAL
        .with_borrow(|state| ic_wire::native::encode_response(CHECKPOINT, state, ARTIFACT_LIMITS))
        .expect("bounded artifact checkpoint");
    let pages = (bytes.len() as u64 + 4).div_ceil(65_536);
    let existing = api::stable_size();
    if pages > existing {
        stable::stable_grow(pages - existing).expect("checkpoint allocation");
    }
    stable::stable_write(
        0,
        &u32::try_from(bytes.len())
            .expect("bounded length")
            .to_le_bytes(),
    );
    stable::stable_write(4, &bytes);
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns argument bytes"
)]
fn decode_upgrade(bytes: Vec<u8>) -> bool {
    ic_wire::native::decode_request(UPGRADE, &bytes, LIMITS).expect("bounded upgrade argument")
}

#[cdk::post_upgrade(decode_with = "decode_upgrade")]
fn post_upgrade(trap_after_restore: bool) {
    assert!(api::stable_size() > 0, "artifact checkpoint missing");
    let mut length = [0; 4];
    stable::stable_read(0, &mut length);
    let length = u32::from_le_bytes(length) as usize;
    assert!(
        length > 0 && length <= ARTIFACT_LIMITS.bytes,
        "checkpoint length outside bound"
    );
    let mut bytes = vec![0; length];
    stable::stable_read(4, &mut bytes);
    let state: Retained = ic_wire::native::decode_response(CHECKPOINT, &bytes, ARTIFACT_LIMITS)
        .expect("valid artifact checkpoint");
    if let Some(approval) = &state.approval {
        approval
            .validate_contents()
            .expect("valid retained artifact");
    }
    JOURNAL.with_borrow_mut(|journal| *journal = state);
    if trap_after_restore {
        api::trap("injected artifact maintenance trap");
    }
}
