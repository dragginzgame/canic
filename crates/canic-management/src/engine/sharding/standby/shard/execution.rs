//! Each uncertain effect resumes by observation under the original approval.
use super::{ChildRetirement, Retirement, RetirementPhase as P, Stage};
use crate::engine::{
    self, STATE, api, delivery, management,
    sharding::{self, standby},
};
use canic::sharding::standby::StandbyPoolError;
use canic::{
    sharding::standby::{Asset, ResetPhase, Source},
    topology::dormancy::{DormancyReceipt, Receipt},
};
use canic_management_protocol::children::ChildId;
use ic_platform_protocol::{management as mgmt, management_chunks::ChunkHash};
use ic_platform_types::Principal;
use ic_wasm_store_protocol as store;

fn check(
    approval: &ChildRetirement,
    subject: Principal,
    grant: impl Fn(Principal) -> Result<(), String>,
) -> Result<(), StandbyPoolError> {
    grant(subject).map_err(StandbyPoolError::CustodyOccupied)?;
    sharding::admit_pool_change(false)
        .map_err(|e| StandbyPoolError::CustodyOccupied(format!("outstanding custody: {e:?}")))?;
    if api::time() >= approval.expires_at_ns {
        return Err(StandbyPoolError::Deadline(
            "retirement approval expired".into(),
        ));
    }
    Ok(())
}

/// Advance one approved shard retirement, observing previously issued effects.
/// # Errors
/// Refuses changed authority, open placements, stale evidence and premature reuse.
/// # Panics
/// Requires intact Root custody.
pub async fn retire(
    input: Stage<ChildRetirement>,
    grant: impl Fn(Principal) -> Result<(), String>,
) -> Result<P, StandbyPoolError> {
    let controller = api::caller();
    if !api::is_controller(&controller) {
        return Err(StandbyPoolError::Unauthorized("controller required".into()));
    }
    let approval = &input.approval;
    let subject = Principal::from_text(&approval.child)
        .map_err(|_| StandbyPoolError::InvalidApproval("invalid shard principal".into()))?;
    let slot = ChildId::placement(approval.slot as usize)
        .ok_or_else(|| StandbyPoolError::InvalidApproval("unknown shard slot".into()))?;
    let _lock = engine::observing(slot).ok_or_else(|| {
        StandbyPoolError::CustodyOccupied("retirement observation in progress".into())
    })?;
    let deadline = engine::role_issuer::deadline(subject, &approval.generation)
        .ok_or_else(|| StandbyPoolError::ProofOutstanding("role issuance remains open".into()))?;
    let (before, hub, old_hash) = STATE.with_borrow(|s| {
        let s = s.as_ref().unwrap();
        let ledger = s
            .pools
            .shard_at(slot)
            .ok_or_else(|| StandbyPoolError::NotConfigured("missing shard pool".into()))?;
        if approval.root != api::canister_self().to_text()
            || approval.child != subject.to_text()
            || s.pools.capacity.is_none()
            || !ChildId::SHARDS.contains(&slot)
            || ledger.principals[slot.index()].as_deref() != Some(&approval.child)
            || s.pools.draining.get(&slot) != Some(&approval.generation)
            || !s.pools.evacuated.contains(&slot)
            || input.reference.store != ledger.approval.reference.store
        {
            return Err(StandbyPoolError::ApprovalConflict(
                "retirement binding differs".into(),
            ));
        }
        let before = match s.pools.retirements.get(&slot) {
            Some(r) if r.input == input && r.proof_deadline == deadline => Some(r.phase),
            Some(_) => {
                return Err(StandbyPoolError::ApprovalConflict(
                    "retirement approval differs".into(),
                ));
            }
            None => None,
        };
        let journal = &s.journal.0[&slot];
        if !journal.ready_for_role() || journal.1.approval.is_some() {
            return Err(StandbyPoolError::CustodyOccupied(
                "shard has outstanding custody".into(),
            ));
        }
        Ok((
            before,
            ledger.approval.hub.parse().expect("approved hub"),
            journal
                .0
                .observed_module
                .clone()
                .ok_or_else(|| StandbyPoolError::WrongAsset("missing shard module".into()))?,
        ))
    })?;
    if before == Some(P::Returned) {
        return Ok(P::Returned);
    }
    let authorize = || {
        if !api::is_controller(&controller) {
            return Err(StandbyPoolError::Unauthorized("controller changed".into()));
        }
        check(approval, subject, &grant)
    };
    let phase = |next| {
        STATE.with_borrow_mut(|s| {
            let state = s.as_mut().unwrap();
            state
                .pools
                .retirements
                .entry(slot)
                .or_insert_with(|| Retirement {
                    input: input.clone(),
                    phase: next,
                    proof_deadline: deadline,
                })
                .phase = next;
            state.pool.set(state.pools.encode());
        })
    };
    let reference = &input.reference;
    delivery::admit(reference, &reference.manifest.hash)
        .map_err(|_| StandbyPoolError::InvalidApproval("invalid dormant release".into()))?;
    let source = delivery::retained(
        slot,
        ic_wire::method_id("canic.root.retire_released_child.v1"),
        &input,
    );
    let args = mgmt::CanisterArgs {
        canister_id: subject,
    }
    .encode(1024)
    .expect("subject");
    let chunks: Vec<_> = reference
        .manifest
        .chunks
        .iter()
        .map(|h| ChunkHash { hash: h.to_vec() })
        .collect();
    let store = reference.store.parse().expect("approved Store");
    let receipt = Receipt {
        root: api::canister_self(),
        subject,
        generation: ic_wire::ByteArray::new(approval.generation),
        proof_deadline_ns: deadline,
    };
    let next = match before {
        None | Some(P::PinIssued) => {
            if before.is_none() {
                authorize()?;
                if approval.expires_at_ns <= deadline
                    || approval.expires_at_ns
                        > deadline.max(api::time()).saturating_add(600_000_000_000)
                {
                    return Err(StandbyPoolError::Deadline(
                        "retirement deadline must allow proof expiry and a bounded return window"
                            .into(),
                    ));
                }
                if reference.manifest.hash.as_slice() == old_hash {
                    return Err(StandbyPoolError::InvalidApproval(
                        "dormant release matches original code".into(),
                    ));
                }
                phase(P::PinIssued);
            }
            let pin = delivery::pin_receipt(&source, before.is_none())
                .await
                .map_err(|e| StandbyPoolError::AwaitingConfirmation(format!("pin: {e:?}")))?
                .map_err(|e| StandbyPoolError::AwaitingConfirmation(format!("pin: {e:?}")))?;
            if pin.pin != source.pin() || pin.owner != api::canister_self().as_slice() {
                return Err(StandbyPoolError::InvalidObservation(
                    "dormant pin differs".into(),
                ));
            }
            P::Pinned
        }
        Some(P::Pinned) => {
            authorize()?;
            let request = store::PrepareCache {
                pin: source.pin(),
                expires_at_ns: approval
                    .expires_at_ns
                    .min(api::time().saturating_add(300_000_000_000)),
            };
            phase(P::CacheIssued);
            let result: Result<store::CacheReady, store::CacheError> =
                delivery::call(&source, "prepare_cache", store::PREPARE_CACHE, &request)
                    .await
                    .map_err(|e| StandbyPoolError::AwaitingConfirmation(format!("cache: {e:?}")))?;
            if result
                .map_err(|e| StandbyPoolError::AwaitingConfirmation(format!("cache: {e:?}")))?
                .pin
                != request.pin
            {
                return Err(StandbyPoolError::InvalidObservation(
                    "dormant cache differs".into(),
                ));
            }
            return Ok(P::CacheIssued);
        }
        Some(P::CacheIssued) => {
            delivery::verify_chunks(store, &chunks).await.map_err(|e| {
                StandbyPoolError::AwaitingConfirmation(format!("cache observation: {e:?}"))
            })?;
            P::Cached
        }
        Some(P::Cached | P::InstallIssued) => {
            if before == Some(P::Cached) {
                authorize()?;
                canic::method::call::<canic::topology::directory::WithdrawChild>(
                    hub,
                    5,
                    &(subject, approval.generation),
                )
                .await
                .map_err(|error| {
                    StandbyPoolError::AwaitingConfirmation(format!(
                        "hub withdrawal unconfirmed: {error:?}"
                    ))
                })?
                .map_err(|e| StandbyPoolError::AwaitingConfirmation(e.to_string()))?;
            }
            let (status, _) = standby::reset::observe_target(&args).await?;
            if before == Some(P::Cached) {
                if status.module_hash != Some(old_hash) {
                    return Err(StandbyPoolError::WrongAsset("source module differs".into()));
                }
                authorize()?;
                let bytes = mgmt::InstallChunkedCodeArgs {
                    mode: mgmt::InstallMode::Reinstall,
                    target_canister: subject,
                    store_canister: Some(store),
                    chunk_hashes_list: chunks,
                    wasm_module_hash: reference.manifest.hash.to_vec(),
                    arg: receipt.encode().expect("receipt"),
                    sender_canister_version: Some(api::canister_version()),
                }
                .encode(8192)
                .expect("install arguments");
                phase(P::InstallIssued);
                management("install_chunked_code", bytes, 0)
                    .await
                    .map_err(|e| {
                        StandbyPoolError::EffectIssued(format!("dormant install issued: {e:?}"))
                    })?;
                return Ok(P::InstallIssued);
            }
            if status.module_hash.as_deref() != Some(&reference.manifest.hash) {
                return Err(StandbyPoolError::AwaitingConfirmation(
                    "dormant installation unconfirmed".into(),
                ));
            }
            if canic::method::call::<DormancyReceipt>(subject, 10, &())
                .await
                .map_err(|e| {
                    StandbyPoolError::AwaitingConfirmation(format!("dormancy receipt: {e:?}"))
                })?
                != receipt
            {
                return Err(StandbyPoolError::InvalidObservation(
                    "dormancy receipt differs".into(),
                ));
            }
            P::Dormant
        }
        Some(current) => {
            if api::time() < deadline {
                return Err(StandbyPoolError::ProofOutstanding(
                    "original role proof window remains open".into(),
                ));
            }
            let prior = match current {
                P::Dormant => None,
                P::StopIssued => Some(ResetPhase::StopIssued),
                P::UninstallIssued => Some(ResetPhase::UninstallIssued),
                P::Uninstalled => Some(ResetPhase::Uninstalled),
                P::StartIssued => Some(ResetPhase::StartIssued),
                _ => unreachable!(),
            };
            let (observed, method, balance) = standby::reset::observe_reset(
                &args,
                prior,
                &reference.manifest.hash,
                approval.minimum_reuse_cycles,
            )
            .await?;
            let next = match observed {
                ResetPhase::StopIssued => P::StopIssued,
                ResetPhase::UninstallIssued => P::UninstallIssued,
                ResetPhase::Uninstalled => P::Uninstalled,
                ResetPhase::StartIssued => P::StartIssued,
                ResetPhase::Returned => P::Returned,
            };
            if let Some(method) = method {
                authorize()?;
                phase(next);
                management(method, args, 0).await.map_err(|e| {
                    StandbyPoolError::EffectIssued(format!("{method} issued: {e:?}"))
                })?;
                return Ok(next);
            }
            if next == P::Returned {
                grant(subject).map_err(StandbyPoolError::CustodyOccupied)?;
                sharding::admit_pool_change(false).map_err(|e| {
                    StandbyPoolError::CustodyOccupied(format!("outstanding custody: {e:?}"))
                })?;
                STATE.with_borrow_mut(|s| {
                    let s = s.as_mut().unwrap();
                    let r = s.pools.retirements.get_mut(&slot).unwrap();
                    r.phase = next;
                    let target = r.target();
                    let balance = balance.expect("observed return balance");
                    if let Some((_, asset)) = s.pools.standby.iter_mut().find(|(_, asset)| {
                        *asset == Asset::Claimed(subject, target.allocation.clone())
                    }) {
                        *asset = Asset::Available(subject, balance);
                    } else {
                        s.pools.standby.push((
                            Source::Shard(target.clone()),
                            Asset::Available(subject, balance),
                        ));
                    }
                    s.pools.returns.push(standby::Returned {
                        target,
                        deadline: approval.expires_at_ns,
                        balance,
                    });
                    standby::log_surplus(&s.pools);
                    s.pool.set(s.pools.encode());
                });
                return Ok(next);
            }
            next
        }
    };
    phase(next);
    Ok(next)
}
