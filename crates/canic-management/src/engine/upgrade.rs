//! Serial child upgrades with bounded immutable history and one active effect owner.
use super::{
    MANAGEMENT_LIMITS, OBSERVING, STATE, State, failure, fault, management, schedule, snapshot,
};
use canic_management_protocol::upgrade::{
    Approval, MAX_UPGRADES, Phase as UpgradePhase, Stage as UpgradeStage, Status as UpgradeStatus,
};
use canic_management_protocol::{Failure, Phase, RESERVE_CYCLES, Refusal, STAGE_LIMITS, digest};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{
        CanisterIdRecord, CanisterInstallMode, CanisterStatusResult, ChunkHash,
        InstallChunkedCodeArgs, UploadChunkArgs,
    },
};
use ic_platform_runtime::{
    Principal,
    call::{Call, WaitPolicy},
    cdk::api,
};

/// Read committed upgrade evidence without dispatching work.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn status() -> UpgradeStatus {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("initialized manager")
            .journal
            .get()
            .1
            .clone()
    })
}
fn mutate(f: impl FnOnce(&mut UpgradeStatus)) {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut value = state.journal.get().clone();
        f(&mut value.1);
        state.journal.set(value);
    });
}
/// Lookup never dispatches, replaces or reopens a historical operation.
/// # Panics
/// Traps if lifecycle initialization has not completed.
#[must_use]
pub fn observe(expected: &Approval) -> UpgradeStatus {
    STATE.with_borrow(|state| {
        let journal = state.as_ref().expect("initialized manager").journal.get();
        journal
            .2
            .iter()
            .chain(std::iter::once(&journal.1))
            .find(|record| record.approval.as_ref() == Some(expected))
            .cloned()
            .unwrap_or_default()
    })
}

pub(super) fn validate(state: &State) {
    let journal = state.journal.get();
    assert!(journal.2.len() < MAX_UPGRADES, "bounded upgrade history");
    let mut previous = journal
        .0
        .approval
        .as_ref()
        .map_or([0; 32], |a| a.module_hash);
    let mut prior: Option<&UpgradeStatus> = None;
    for record in journal.2.iter().chain(std::iter::once(&journal.1)) {
        validate_record(record, &journal.0, &previous);
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
            digest(&state.artifact.get().1),
            a.module_hash,
            "current artifact custody"
        );
    } else {
        assert!(state.artifact.get().1.is_empty());
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
        if s.attempts[0] == 1 {
            assert!(s.previous_version.is_some(), "old version custody");
        }
        if s.attempts[1] == 1 {
            assert_eq!(s.attempts[0], 1, "upload precedes upgrade");
        }
        if matches!(s.phase, UpgradePhase::Verified | UpgradePhase::Ready) {
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
pub fn stage(input: UpgradeStage) -> Result<UpgradeStatus, Refusal> {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    let retained = observe(&input.approval);
    if retained.approval.is_some() {
        return if digest(&input.wasm) == input.approval.module_hash {
            Ok(retained)
        } else {
            Err(Refusal::Artifact)
        };
    }
    let current = status();
    if OBSERVING.get()
        || (current.approval.is_some()
            && (current.phase != UpgradePhase::Ready || current.failure.is_some()))
    {
        return Err(Refusal::Occupied);
    }
    let full = STATE.with_borrow(|state| {
        let journal = state.as_ref().expect("initialized manager").journal.get();
        journal.2.len() + usize::from(journal.1.approval.is_some()) >= MAX_UPGRADES
    });
    if full {
        return Err(Refusal::Occupied);
    }
    let creation = snapshot();
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
        .is_some_and(|old| input.approval.execute_at_ns <= old.execute_at_ns)
    {
        return Err(Refusal::Occupied);
    }
    input.validate(
        &api::canister_self().to_text(),
        creation.worker.as_deref().expect("created child"),
        &previous,
        api::time(),
    )?;
    if api::canister_liquid_cycle_balance() < u128::from(RESERVE_CYCLES) {
        return Err(Refusal::Reserve);
    }
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut artifacts = state.artifact.get().clone();
        artifacts.1 = input.wasm;
        state.artifact.set(artifacts);
    });
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized manager");
        let mut journal = state.journal.get().clone();
        if journal.1.approval.is_some() {
            journal.2.push(journal.1.clone());
        }
        journal.1 = UpgradeStatus {
            approval: Some(input.approval),
            phase: UpgradePhase::Scheduled,
            ..Default::default()
        };
        state.journal.set(journal);
    });
    schedule();
    Ok(status())
}
pub(super) fn next() -> Option<u64> {
    let s = status();
    if s.failure.is_some() {
        return None;
    }
    match s.phase {
        UpgradePhase::Scheduled => Some(s.approval.expect("approval").execute_at_ns),
        UpgradePhase::Checked
        | UpgradePhase::Uploaded
        | UpgradePhase::Installed
        | UpgradePhase::Verified => Some(api::time()),
        _ => None,
    }
}
async fn inspect(a: &Approval) -> Result<CanisterStatusResult, Failure> {
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
    if value.settings.controllers != [api::canister_self()] {
        return Err(Failure::Mismatch);
    }
    Ok(value)
}
fn issue(index: usize, phase: UpgradePhase, bytes: &[u8]) {
    mutate(|s| {
        assert_eq!(s.attempts[index], 0, "no effect replay");
        s.attempts[index] = 1;
        s.requests[index] = Some(digest(bytes));
        s.phase = phase;
    });
}
pub(super) async fn step() {
    let s = status();
    let Some(a) = &s.approval else {
        return;
    };
    if matches!(
        s.phase,
        UpgradePhase::Scheduled | UpgradePhase::Checked | UpgradePhase::Uploaded
    ) {
        if api::time() > a.expires_at_ns {
            mutate(|s| s.phase = UpgradePhase::Expired);
            return;
        }
        if api::time() < a.execute_at_ns {
            return;
        }
        if api::canister_liquid_cycle_balance() < u128::from(RESERVE_CYCLES) {
            mutate(|s| s.phase = UpgradePhase::ReserveBlocked);
            return;
        }
    }
    let result = match s.phase {
        UpgradePhase::Scheduled => inspect(a).await.and_then(|value| {
            if value.module_hash.as_deref() != Some(a.previous_module.as_slice())
                || value.version == u64::MAX
                || STATE.with_borrow(|state| {
                    state
                        .as_ref()
                        .expect("initialized manager")
                        .journal
                        .get()
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
            mutate(|s| {
                s.previous_version = Some(value.version);
                s.phase = UpgradePhase::Checked;
            });
            Ok(())
        }),
        UpgradePhase::Checked => {
            let wasm =
                STATE.with_borrow(|state| state.as_ref().expect("state").artifact.get().1.clone());
            let bytes = candid_codec::encode_one(
                &UploadChunkArgs {
                    canister_id: api::canister_self(),
                    chunk: wasm,
                },
                STAGE_LIMITS.bytes,
            )
            .expect("upload args");
            issue(0, UpgradePhase::UploadIssued, &bytes);
            let result = management("upload_chunk", bytes, 0)
                .await
                .and_then(|bytes| {
                    let hash: ChunkHash = decoder::decode_one(&bytes, MANAGEMENT_LIMITS)
                        .map_err(|_| Failure::InvalidReply)?;
                    if hash.hash != a.module_hash {
                        return Err(Failure::Mismatch);
                    }
                    mutate(|s| s.phase = UpgradePhase::Uploaded);
                    Ok(())
                });
            fault(6); // Lost upload callback is recoverable from the chunk store.
            result
        }
        UpgradePhase::Uploaded => {
            match inspect(a).await {
                Err(error) => Err(error),
                Ok(value)
                    if value.module_hash.as_deref() != Some(a.previous_module.as_slice())
                        || Some(value.version) != s.previous_version =>
                {
                    Err(Failure::Mismatch)
                }
                Ok(_) => {
                    // Inspection awaits: check the original deadline/reserve again at dispatch.
                    if api::time() > a.expires_at_ns {
                        mutate(|s| s.phase = UpgradePhase::Expired);
                        return;
                    }
                    if api::canister_liquid_cycle_balance() < u128::from(RESERVE_CYCLES) {
                        mutate(|s| s.phase = UpgradePhase::ReserveBlocked);
                        return;
                    }
                    let bytes = candid_codec::encode_one(
                        &InstallChunkedCodeArgs {
                            mode: CanisterInstallMode::Upgrade(None),
                            target_canister: Principal::from_text(&a.child)
                                .expect("retained child"),
                            store_canister: Some(api::canister_self()),
                            chunk_hashes_list: vec![ChunkHash {
                                hash: a.module_hash.to_vec(),
                            }],
                            wasm_module_hash: a.module_hash.to_vec(),
                            arg: vec![],
                            sender_canister_version: Some(api::canister_version()),
                        },
                        4096,
                    )
                    .expect("upgrade args");
                    issue(1, UpgradePhase::InstallIssued, &bytes);
                    fault(7); // Trap rolls back dispatch and intent together.
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
                                mutate(|s| s.phase = UpgradePhase::Installed);
                                Ok(())
                            });
                    fault(8); // The upgrade may commit while its callback evidence rolls back.
                    result
                }
            }
        }
        UpgradePhase::Installed | UpgradePhase::Verified => observation().await,
        _ => Ok(()),
    };
    if let Err(error) = result {
        mutate(|s| s.failure = Some(error));
    }
}
async fn observation() -> Result<(), Failure> {
    let s = status();
    let Some(a) = &s.approval else {
        return Ok(());
    };
    match s.phase {
        UpgradePhase::UploadIssued => {
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
            if !chunks.iter().any(|c| c.hash == a.module_hash) {
                return Err(Failure::Mismatch);
            }
            mutate(|s| {
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
            mutate(|s| {
                s.phase = UpgradePhase::Verified;
                s.observed_version = Some(value.version);
                s.failure = None;
            });
        }
        UpgradePhase::Verified | UpgradePhase::Ready => {
            let readiness =
                STATE.with_borrow(|state| state.as_ref().expect("state").readiness.clone());
            let reply = Call::new(
                Principal::from_text(&a.child).expect("retained child"),
                readiness.method,
                WaitPolicy::Bounded { timeout_seconds: 5 },
            )
            .take_raw_args(readiness.request)
            .execute()
            .await
            .map_err(failure)?;
            (readiness.accept)(&reply)?;
            mutate(|s| {
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
pub async fn reconcile(expected: canic_management_protocol::upgrade::Approval) -> UpgradeStatus {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    let retained = observe(&expected);
    assert_eq!(
        retained.approval.as_ref(),
        Some(&expected),
        "reconciliation approval mismatch"
    );
    if status().approval.as_ref() != Some(&expected) {
        return retained;
    }
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
