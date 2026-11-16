//! Single-use installation/start/activation under the surviving Root journal. No payload storage.
use super::{
    Approval, Failure, MANAGEMENT_LIMITS, Phase, Refusal, Status, admission, admission_frame, api,
    args, candid_codec, change, decoder, digest, dispatch, gate, inspect, inspect_module,
    management, observe_attempt, preservation, selected, store, target,
};
use canic_management_protocol::store_maintenance::{Install, Installation, MAX_MODULE_BYTES};
use ic_platform::management_canister_types::{
    CanisterInstallMode, CanisterStatusType, ChunkHash, InstallChunkedCodeArgs, UploadChunkArgs,
};

pub(super) fn validate(record: &Status) {
    let issued = matches!(
        record.phase,
        Phase::UploadIssued
            | Phase::Uploaded
            | Phase::InstallIssued
            | Phase::Installed
            | Phase::StartIssued
            | Phase::Verifying
            | Phase::Verified
            | Phase::ActivateIssued
            | Phase::Ready
    );
    assert_eq!(
        record.installation.is_some(),
        issued,
        "installation custody"
    );
    if let Some(value) = &record.installation {
        assert!(
            record
                .preservation
                .as_ref()
                .is_some_and(|scan| scan.complete)
        );
        assert_ne!(value.upload_request, [0; 32]);
        let installed = !matches!(record.phase, Phase::UploadIssued | Phase::Uploaded);
        assert_eq!(value.request.is_some(), installed);
        assert_eq!(value.sender_version.is_some(), installed);
        if let Some(sender_version) = value.sender_version {
            assert_eq!(
                value.request,
                Some(digest(&install_args(&record.approval, sender_version)))
            );
        }
        assert_eq!(
            value.start_request.is_some(),
            !matches!(
                record.phase,
                Phase::UploadIssued | Phase::Uploaded | Phase::InstallIssued | Phase::Installed
            )
        );
        if let Some(request) = value.start_request {
            assert_eq!(request, digest(&args(&record.approval)));
        }
        assert_eq!(
            value.activation_request.is_some(),
            matches!(record.phase, Phase::ActivateIssued | Phase::Ready)
        );
        if let Some(request) = value.activation_request {
            assert_eq!(
                request,
                digest(&admission_frame(&store::AdmissionCommand::Activate(
                    record.approval.attempt.clone()
                )))
            );
        }
        if matches!(
            record.phase,
            Phase::Verified | Phase::ActivateIssued | Phase::Ready
        ) {
            assert_eq!(
                value.verification, record.preservation,
                "replacement preservation"
            );
        }
    }
}

/// Install the explicitly supplied approved Wasm once. An issued attempt is never repeated.
/// # Errors
/// Refuses mismatched/oversized bytes, missing drain/preservation evidence or an unready phase.
/// # Panics
/// Traps for invalid authority/lifecycle, failed persistence or injected qualification faults.
pub async fn install(input: Install) -> Result<Status, Refusal> {
    let record = selected(&input.approval);
    if input.wasm.len() > MAX_MODULE_BYTES
        || !input.wasm.starts_with(b"\0asm\x01\0\0\0")
        || digest(&input.wasm) != record.approval.module_hash
    {
        return Err(Refusal::Artifact);
    }
    if record.installation.is_some() {
        return Ok(record);
    }
    if record.phase != Phase::Stopped || !record.preservation.as_ref().is_some_and(|p| p.complete) {
        return Err(Refusal::Occupied);
    }
    if let Err(error) = issue(&record, input.wasm).await {
        change(&record.approval, Phase::Stopped, |value| {
            value.failure = Some(error)
        });
    }
    Ok(observe_attempt(&record.approval).expect("retained maintenance"))
}
async fn issue(record: &Status, wasm: Vec<u8>) -> Result<(), Failure> {
    if inspect(&record.approval).await?.status != CanisterStatusType::Stopped {
        return Err(Failure::Mismatch);
    }
    dispatch(record)?;
    let bytes = candid_codec::encode_one(
        &UploadChunkArgs {
            canister_id: target(&record.approval),
            chunk: wasm,
        },
        MAX_MODULE_BYTES + 4096,
    )
    .expect("Store install args");
    change(&record.approval, Phase::Stopped, |value| {
        value.phase = Phase::UploadIssued;
        value.failure = None;
        value.installation = Some(Installation {
            upload_request: digest(&bytes),
            request: None,
            sender_version: None,
            start_request: None,
            verification: record.preservation.empty(),
            activation_request: None,
        });
    });
    let result = management("upload_chunk", bytes, 0).await;
    super::super::fault(super::super::ChildId::FIRST, 15); // Callback trap cannot erase issued intent.
    let result = result.and_then(|reply| {
        let chunk: ChunkHash =
            decoder::decode_one(&reply, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)?;
        if chunk.hash != record.approval.module_hash {
            return Err(Failure::Mismatch);
        }
        Ok(())
    });
    match result {
        Ok(()) => change(&record.approval, Phase::UploadIssued, |value| {
            value.phase = Phase::Uploaded;
            value.failure = None;
        }),
        Err(error) => change(&record.approval, Phase::UploadIssued, |value| {
            value.failure = Some(error)
        }),
    }
    Ok(())
}
fn install_args(approval: &Approval, sender_version: u64) -> Vec<u8> {
    candid_codec::encode_one(
        &InstallChunkedCodeArgs {
            mode: CanisterInstallMode::Upgrade(None),
            target_canister: target(approval),
            store_canister: Some(target(approval)),
            chunk_hashes_list: vec![ChunkHash {
                hash: approval.module_hash.to_vec(),
            }],
            wasm_module_hash: approval.module_hash.to_vec(),
            arg: vec![],
            sender_canister_version: Some(sender_version),
        },
        4096,
    )
    .expect("Store install args")
}
async fn chunk_present(approval: &Approval) -> Result<(), Failure> {
    let reply = management("stored_chunks", args(approval), 0).await?;
    let chunks: Vec<ChunkHash> =
        decoder::decode_one(&reply, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)?;
    if chunks
        .iter()
        .any(|chunk| chunk.hash == approval.module_hash)
    {
        Ok(())
    } else {
        Err(Failure::Uncertain)
    }
}
fn empty_reply(result: Result<Vec<u8>, Failure>) -> Result<(), Failure> {
    result.and_then(|reply| {
        decoder::decode_args::<()>(&reply, MANAGEMENT_LIMITS, decoder::Arity::Exact)
            .map_err(|_| Failure::InvalidReply)
    })
}

pub(super) async fn advance(record: &Status) -> Result<(), Failure> {
    let approval = &record.approval;
    match record.phase {
        Phase::UploadIssued => {
            if inspect(approval).await?.status != CanisterStatusType::Stopped {
                return Err(Failure::Mismatch);
            }
            chunk_present(approval).await?;
            change(&record.approval, Phase::UploadIssued, |value| {
                value.phase = Phase::Uploaded;
                value.failure = None;
            });
        }
        Phase::Uploaded => {
            if inspect(approval).await?.status != CanisterStatusType::Stopped {
                return Err(Failure::Mismatch);
            }
            chunk_present(approval).await?;
            dispatch(record)?;
            let sender_version = api::canister_version();
            let bytes = install_args(approval, sender_version);
            change(&record.approval, Phase::Uploaded, |value| {
                let installation = value.installation.as_mut().expect("issued upload");
                installation.request = Some(digest(&bytes));
                installation.sender_version = Some(sender_version);
                value.phase = Phase::InstallIssued;
                value.failure = None;
            });
            super::super::fault(super::super::ChildId::FIRST, 19); // Trap rolls back intent and enqueue together.
            let result = empty_reply(management("install_chunked_code", bytes, 0).await);
            super::super::fault(super::super::ChildId::FIRST, 16);
            if let Err(error) = result {
                change(&record.approval, Phase::InstallIssued, |value| {
                    value.failure = Some(error)
                });
            }
        }
        Phase::InstallIssued => {
            if inspect_module(approval, approval.module_hash).await?.status
                != CanisterStatusType::Stopped
            {
                return Err(Failure::Mismatch);
            }
            change(&record.approval, Phase::InstallIssued, |value| {
                value.phase = Phase::Installed;
                value.failure = None;
            });
        }
        Phase::Installed => {
            if inspect_module(approval, approval.module_hash).await?.status
                != CanisterStatusType::Stopped
            {
                return Err(Failure::Mismatch);
            }
            dispatch(record)?;
            let bytes = args(approval);
            change(&record.approval, Phase::Installed, |value| {
                value
                    .installation
                    .as_mut()
                    .expect("issued install")
                    .start_request = Some(digest(&bytes));
                value.phase = Phase::StartIssued;
                value.failure = None;
            });
            let result = empty_reply(management("start_canister", bytes, 0).await);
            super::super::fault(super::super::ChildId::FIRST, 17);
            if let Err(error) = result {
                change(&record.approval, Phase::StartIssued, |value| {
                    value.failure = Some(error)
                });
            }
        }
        Phase::StartIssued => {
            if inspect_module(approval, approval.module_hash).await?.status
                != CanisterStatusType::Running
            {
                return Err(Failure::Mismatch);
            }
            gate(approval, store::AdmissionCommand::Observe).await?;
            change(&record.approval, Phase::StartIssued, |value| {
                value.phase = Phase::Verifying;
                value.failure = None;
            });
        }
        Phase::Verifying => preservation::capture(record).await?,
        Phase::ActivateIssued => {
            if inspect_module(approval, approval.module_hash).await?.status
                != CanisterStatusType::Running
            {
                return Err(Failure::Mismatch);
            }
            admission(approval, store::AdmissionCommand::Observe, false).await?;
            change(&record.approval, Phase::ActivateIssued, |value| {
                value.phase = Phase::Ready;
                value.failure = None;
            });
        }
        Phase::Verified | Phase::Ready => (), // Neither observation nor advance implicitly activates.
        _ => unreachable!("installation continuation"),
    }
    Ok(())
}

/// Explicitly activate the exact verified replacement; lost replies recover through advance.
/// # Panics
/// Traps for mismatched authority/lifecycle, failed writes or injected qualification faults.
pub async fn activate(approval: Approval) -> Status {
    let record = selected(&approval);
    if record.phase != Phase::Verified {
        return record;
    }
    if let Err(error) = activate_once(&record).await {
        change(&record.approval, Phase::Verified, |value| {
            value.failure = Some(error)
        });
    }
    observe_attempt(&record.approval).expect("retained maintenance")
}
async fn activate_once(record: &Status) -> Result<(), Failure> {
    let approval = &record.approval;
    if inspect_module(approval, approval.module_hash).await?.status != CanisterStatusType::Running {
        return Err(Failure::Mismatch);
    }
    gate(approval, store::AdmissionCommand::Observe).await?;
    dispatch(record)?;
    let command = store::AdmissionCommand::Activate(approval.attempt.clone());
    change(&record.approval, Phase::Verified, |value| {
        value
            .installation
            .as_mut()
            .expect("issued install")
            .activation_request = Some(digest(&admission_frame(&command)));
        value.phase = Phase::ActivateIssued;
        value.failure = None;
    });
    let result = admission(approval, command, false).await;
    super::super::fault(super::super::ChildId::FIRST, 18);
    if let Err(error) = result {
        change(&record.approval, Phase::ActivateIssued, |value| {
            value.failure = Some(error)
        });
    }
    Ok(())
}
