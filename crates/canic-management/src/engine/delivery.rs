//! Artifact delivery under the existing child journal. Store bytes never enter Root.
use super::{MANAGEMENT_LIMITS, STATE, State, failure, management, response};
use crate::storage::Artifact;
use canic_management_protocol::{
    Failure, Refusal, children::ChildId, stored, stored::STORE_LIMITS,
};
use ic_platform_protocol::{
    management::CanisterArgs,
    management_chunks::{self as chunks, ChunkHash},
};
use ic_platform_runtime::{
    call::{Call, Wait},
    system as api,
};
use ic_platform_types::Principal;
use ic_wasm_store_protocol as store;
use serde::{Serialize, de::DeserializeOwned};

pub(super) fn source(slot: ChildId, upgrade: bool) -> Option<stored::Delivery> {
    STATE.with_borrow(|state| {
        let sources = state
            .as_ref()
            .expect("open engine")
            .artifact
            .0
            .get(&slot)?
            .sources()?;
        if upgrade {
            sources.upgrades.last()
        } else {
            Some(&sources.creation)
        }
        .cloned()
    })
}

pub(super) fn is_stored(slot: ChildId) -> bool {
    source(slot, false).is_some()
}

fn mutate(slot: ChildId, upgrade: bool, f: impl FnOnce(&mut stored::Delivery)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open engine");
        let mut artifacts = state.artifact.clone();
        let sources = match artifacts.0.get_mut(&slot).expect("known child") {
            Artifact::Stored(sources) | Artifact::Application(sources, _) => sources,
            Artifact::Empty => panic!("Store source required"),
        };
        f(if upgrade {
            sources.upgrades.last_mut().expect("upgrade source")
        } else {
            &mut sources.creation
        });
        state.set_artifact(artifacts);
    });
}

pub(super) fn admit(reference: &stored::Reference, hash: &[u8; 32]) -> Result<(), Refusal> {
    let principal = Principal::from_text(&reference.store).map_err(|_| Refusal::Binding)?;
    if principal == Principal::anonymous()
        || principal == Principal::management_canister()
        || principal == api::canister_self()
        || principal.to_text() != reference.store
    {
        return Err(Refusal::Binding);
    }
    if !reference.manifest.valid() || reference.manifest.hash != *hash {
        return Err(Refusal::Artifact);
    }
    Ok(())
}

pub(super) fn retained<A: Serialize>(
    slot: ChildId,
    method: u64,
    input: &stored::Stage<A>,
) -> stored::Delivery {
    stored::Delivery {
        reference: input.reference.clone(),
        operation: input.pin(method, slot).operation,
        state: stored::PinState::Pending,
        cache_deadline: None,
    }
}

pub(super) fn validate(state: &State, slot: ChildId) {
    let Some(sources) = state.artifact.0[&slot].sources() else {
        assert!(
            state.journal.0[&slot].0.approval.is_none(),
            "missing child source"
        );
        return;
    };
    let journal = &state.journal.0[&slot];
    let creation = journal
        .0
        .approval
        .as_ref()
        .expect("Store creation approval");
    let creation_input = stored::Stage {
        approval: creation,
        reference: sources.creation.reference.clone(),
    };
    let pin = if let Artifact::Application(_, lifecycle) = &state.artifact.0[&slot] {
        #[cfg(not(feature = "blob"))]
        assert!(
            lifecycle.upgrades.is_empty(),
            "Blob maintenance requires its selected runtime feature"
        );
        assert_eq!(
            lifecycle.upgrades.len(),
            sources.upgrades.len(),
            "complete lifecycle history"
        );
        assert!(
            !lifecycle.creation.as_slice().is_empty(),
            "explicit initialization"
        );
        canic_management_protocol::application::Stage {
            source: creation_input,
            arguments: lifecycle.creation.clone(),
        }
        .pin(canic_management_protocol::application::STAGE, slot)
    } else {
        creation_input.pin(stored::STAGE, slot)
    };
    validate_source(
        &sources.creation,
        &pin,
        creation.module_hash,
        creation.expires_at_ns,
        journal.0.attempts[1],
    );
    if journal.0.attempts[0] == 1 {
        assert_eq!(sources.creation.state, stored::PinState::Retained);
    }
    let upgrades: Vec<_> = journal
        .2
        .iter()
        .chain(std::iter::once(&journal.1))
        .filter(|s| s.approval.is_some())
        .collect();
    assert_eq!(
        sources.upgrades.len(),
        upgrades.len(),
        "complete Store history"
    );
    for (index, (source, status)) in sources.upgrades.iter().zip(upgrades).enumerate() {
        let approval = status.approval.as_ref().expect("upgrade approval");
        let input = stored::Stage {
            approval: approval.clone(),
            reference: source.reference.clone(),
        };
        let pin = if let Artifact::Application(_, lifecycle) = &state.artifact.0[&slot] {
            {
                let maintenance = &lifecycle.upgrades[index];
                canic_management_protocol::application::Upgrade {
                    source: canic_management_protocol::application::Stage {
                        source: input,
                        arguments: maintenance.arguments.clone(),
                    },
                    attempt: maintenance.attempt,
                }
                .pin(slot)
            }
        } else {
            input.pin(stored::STAGE_UPGRADE, slot)
        };
        validate_source(
            source,
            &pin,
            approval.module_hash,
            approval.expires_at_ns,
            status.attempts[0],
        );
    }
}

fn validate_source(
    source: &stored::Delivery,
    pin: &store::Pin,
    hash: [u8; 32],
    expires: u64,
    uploads: u8,
) {
    admit(&source.reference, &hash).expect("Store source binding");
    assert_eq!(&source.pin(), pin, "retention identity");
    assert_eq!(
        source.cache_deadline.is_some(),
        uploads == 1,
        "cache dispatch custody"
    );
    if let Some(deadline) = source.cache_deadline {
        assert!(deadline <= expires, "original authority bounds cache work");
        assert_eq!(source.state, stored::PinState::Retained);
    }
}

pub(super) async fn call<T: Serialize, R: DeserializeOwned>(
    source: &stored::Delivery,
    method: &str,
    id: u64,
    input: &T,
) -> Result<R, Failure> {
    let bytes = ic_wire::encode(
        input,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .expect("Store request");
    let reply = Call::new(
        Principal::from_text(&source.reference.store).expect("Store principal"),
        method,
        Wait::Unbounded,
    )
    .owned_args(bytes)
    .execute()
    .await
    .map_err(failure)
    .and_then(response)?;
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        STORE_LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)
}

/// Pending pins dispatch once. Issued pins can only be reconstructed by observation.
pub(super) async fn pin(slot: ChildId, upgrade: bool) -> Result<(), Failure> {
    let source = source(slot, upgrade).ok_or(Failure::Mismatch)?;
    if source.state == stored::PinState::Retained {
        return Ok(());
    }
    let issue = source.state == stored::PinState::Pending;
    if issue {
        mutate(slot, upgrade, |s| s.state = stored::PinState::Issued);
    }
    let receipt = pin_receipt(&source, issue)
        .await?
        .map_err(|_| Failure::Mismatch)?;
    if receipt.pin != source.pin() || receipt.owner != api::canister_self().as_slice() {
        return Err(Failure::Mismatch);
    }
    mutate(slot, upgrade, |s| s.state = stored::PinState::Retained);
    Ok(())
}

pub(super) async fn pin_receipt(
    source: &stored::Delivery,
    issue: bool,
) -> Result<Result<store::PinReceipt, store::Error>, Failure> {
    if issue {
        return call(source, "pin", store::PIN, &source.pin()).await;
    }
    let result: Result<store::Observation, store::Error> = call(
        source,
        "observe",
        store::OBSERVE,
        &store::Observe::Pin(source.pin()),
    )
    .await?;
    match result {
        Ok(store::Observation::Pin(receipt)) => Ok(Ok(receipt)),
        Ok(_) => Err(Failure::InvalidReply),
        Err(error) => Ok(Err(error)),
    }
}

pub(super) fn needs_pin(slot: ChildId, upgrade: bool) -> bool {
    source(slot, upgrade).is_some_and(|s| s.state != stored::PinState::Retained)
}

/// Retain the exact upload frame before the IC call.
pub(super) async fn dispatch_upload(
    slot: ChildId,
    upgrade: bool,
    hash: [u8; 32],
    expires: u64,
    issue: impl FnOnce(&[u8]),
) -> Result<(), Failure> {
    let source = source(slot, upgrade).ok_or(Failure::Mismatch)?;
    if source.reference.manifest.hash != hash {
        return Err(Failure::Mismatch);
    }
    let bytes = {
        assert_eq!(source.state, stored::PinState::Retained);
        assert!(source.cache_deadline.is_none(), "one cache preparation");
        let deadline = expires.min(api::time().saturating_add(300_000_000_000));
        mutate(slot, upgrade, |s| s.cache_deadline = Some(deadline));
        ic_wire::encode(
            &store::PrepareCache {
                pin: source.pin(),
                expires_at_ns: deadline,
            },
            ic_wire::Encoding::Frame {
                method: store::PREPARE_CACHE,
                direction: ic_wire::Direction::Request,
            },
            STORE_LIMITS,
        )
        .expect("Store cache request")
    };
    issue(&bytes);
    let reply = Call::new(
        Principal::from_text(&source.reference.store).expect("Store principal"),
        "prepare_cache",
        Wait::Unbounded,
    )
    .owned_args(bytes)
    .execute()
    .await
    .map_err(failure)
    .and_then(response)?;
    let ready: Result<store::CacheReady, store::CacheError> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: store::PREPARE_CACHE,
            direction: ic_wire::Direction::Response,
        },
        STORE_LIMITS,
    )
    .map_err(|_| Failure::InvalidReply)?;
    let ready = ready.map_err(|_| Failure::Uncertain)?;
    if ready.pin != source.pin() || ready.uploaded as usize > store::MAX_CHUNKS {
        return Err(Failure::Mismatch);
    }
    Ok(())
}

pub(super) async fn install(
    slot: ChildId,
    mode: ic_platform_protocol::management::InstallMode,
    target: Principal,
    hash: [u8; 32],
    issue: impl FnOnce(&[u8]),
) -> Result<(), Failure> {
    use ic_platform_protocol::management::{self as mgmt, InstallChunkedCodeArgs, InstallMode};
    let upgrade = matches!(mode, InstallMode::Upgrade(_));
    let (store, chunks) = layout(slot, upgrade, hash)?;
    let bytes = InstallChunkedCodeArgs {
        mode,
        target_canister: target,
        store_canister: Some(store),
        chunk_hashes_list: chunks,
        wasm_module_hash: hash.to_vec(),
        arg: super::application::arguments(slot, upgrade),
        sender_canister_version: Some(api::canister_version()),
    }
    .encode(8192)
    .expect("bounded chunk install");
    issue(&bytes);
    let reply = management("install_chunked_code", bytes, 0).await?;
    mgmt::decode_empty_response(&reply, MANAGEMENT_LIMITS).map_err(|_| Failure::InvalidReply)
}

fn layout(
    slot: ChildId,
    upgrade: bool,
    hash: [u8; 32],
) -> Result<(Principal, Vec<ChunkHash>), Failure> {
    let source = source(slot, upgrade).ok_or(Failure::Mismatch)?;
    if source.reference.manifest.hash != hash {
        return Err(Failure::Mismatch);
    }
    Ok((
        Principal::from_text(&source.reference.store).expect("Store principal"),
        source
            .reference
            .manifest
            .chunks
            .into_iter()
            .map(|hash| ChunkHash {
                hash: hash.to_vec(),
            })
            .collect(),
    ))
}

pub(super) async fn observe_cache(
    slot: ChildId,
    upgrade: bool,
    hash: [u8; 32],
) -> Result<(), Failure> {
    let (canister_id, expected) = layout(slot, upgrade, hash)?;
    verify_chunks(canister_id, &expected).await
}

pub(super) async fn verify_chunks(
    canister_id: Principal,
    expected: &[ChunkHash],
) -> Result<(), Failure> {
    let bytes = CanisterArgs { canister_id }
        .encode(1024)
        .expect("cache observation");
    let reply = management("stored_chunks", bytes, 0).await?;
    let chunks = chunks::decode_stored_chunks(&reply, MANAGEMENT_LIMITS)
        .map_err(|_| Failure::InvalidReply)?;
    if expected
        .iter()
        .any(|expected| !chunks.iter().any(|chunk| chunk.hash == expected.hash))
    {
        return Err(Failure::Mismatch);
    }
    Ok(())
}
