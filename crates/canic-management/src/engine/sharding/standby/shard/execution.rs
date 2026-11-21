//! Each uncertain effect resumes by observation under the original approval.
use super::{Retirement, RetirementPhase as P, ShardRetirement, Stage};
use crate::engine::{
    self, STATE, api, delivery, management,
    sharding::{self, standby},
};
use canic::{
    sharding::standby::{Asset, ResetPhase, Source},
    topology::dormancy::{DormancyReceipt, Receipt},
};
use canic_management_protocol::children::ChildId;
use ic_platform_protocol::{management as mgmt, management_chunks::ChunkHash};
use ic_platform_types::Principal;
use ic_wasm_store_protocol as store;

fn check(
    approval: &ShardRetirement,
    subject: Principal,
    grant: impl Fn(Principal) -> Result<(), String>,
) -> Result<(), String> {
    grant(subject)?;
    sharding::admit_pool_change(false).map_err(|e| format!("outstanding custody: {e:?}"))?;
    if api::time() >= approval.expires_at_ns {
        return Err("retirement approval expired".into());
    }
    Ok(())
}

/// Advance one approved shard retirement, observing previously issued effects.
/// # Errors
/// Refuses changed authority, open placements, stale evidence and premature reuse.
/// # Panics
/// Requires intact Root custody.
pub async fn retire(
    input: Stage<ShardRetirement>,
    grant: impl Fn(Principal) -> Result<(), String>,
) -> Result<P, String> {
    let controller = api::caller();
    if !api::is_controller(&controller) {
        return Err("controller required".into());
    }
    let approval = &input.approval;
    let subject = Principal::from_text(&approval.shard).map_err(|_| "invalid shard principal")?;
    let slot = ChildId::placement(approval.slot as usize).ok_or("unknown shard slot")?;
    let _lock = engine::observing(slot).ok_or("retirement observation in progress")?;
    let deadline = engine::role_issuer::deadline(subject, &approval.generation)
        .ok_or("role issuance remains open")?;
    let (before, old_hash) = STATE.with_borrow(|s| {
        let s = s.as_ref().unwrap();
        let ledger = s.pools.shards.as_ref().ok_or("missing shard pool")?;
        if approval.root != api::canister_self().to_text()
            || approval.shard != subject.to_text()
            || s.pools.capacity.is_none()
            || !ChildId::SHARDS.contains(&slot)
            || ledger.principals[slot.index()].as_deref() != Some(&approval.shard)
            || s.pools.draining.get(&slot) != Some(&approval.generation)
            || ledger.counts[slot.index()] != 0
            || input.reference.store != ledger.approval.reference.store
        {
            return Err("retirement binding differs");
        }
        let before = match s.pools.retirements.get(&slot) {
            Some(r) if r.input == input && r.proof_deadline == deadline => Some(r.phase),
            Some(_) => return Err("retirement approval differs"),
            None => None,
        };
        let journal = &s.journal.0[&slot];
        if !journal.ready_for_role() || journal.1.approval.is_some() {
            return Err("shard has outstanding custody");
        }
        Ok((
            before,
            journal
                .0
                .observed_module
                .clone()
                .ok_or("missing shard module")?,
        ))
    })?;
    if before == Some(P::Returned) {
        return Ok(P::Returned);
    }
    let authorize = || {
        if !api::is_controller(&controller) {
            return Err("controller changed".into());
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
    delivery::admit(reference, &reference.manifest.hash).map_err(|_| "invalid dormant release")?;
    let source = delivery::retained(slot, ic_wire::method_id("retire_shard"), &input);
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
                    return Err(
                        "retirement deadline must allow proof expiry and a bounded return window"
                            .into(),
                    );
                }
                if reference.manifest.hash.as_slice() == old_hash {
                    return Err("dormant release matches original code".into());
                }
                phase(P::PinIssued);
            }
            let (method, id) = if before.is_none() {
                ("pin", store::PIN)
            } else {
                ("observe_pin", store::OBSERVE_PIN)
            };
            let result: Result<store::PinReceipt, store::Error> =
                delivery::call(&source, method, id, &source.pin())
                    .await
                    .map_err(|e| format!("pin: {e:?}"))?;
            let pin = result.map_err(|e| format!("pin: {e:?}"))?;
            if pin.pin != source.pin() || pin.owner != api::canister_self().as_slice() {
                return Err("dormant pin differs".into());
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
                    .map_err(|e| format!("cache: {e:?}"))?;
            if result.map_err(|e| format!("cache: {e:?}"))?.pin != request.pin {
                return Err("dormant cache differs".into());
            }
            return Ok(P::CacheIssued);
        }
        Some(P::CacheIssued) => {
            delivery::verify_chunks(store, &chunks)
                .await
                .map_err(|e| format!("cache observation: {e:?}"))?;
            P::Cached
        }
        Some(P::Cached | P::InstallIssued) => {
            let (status, _) = standby::reset::observe_target(&args).await?;
            if before == Some(P::Cached) {
                if status.module_hash != Some(old_hash) {
                    return Err("source module differs".into());
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
                    .map_err(|e| format!("dormant install issued: {e:?}"))?;
                return Ok(P::InstallIssued);
            }
            if status.module_hash.as_deref() != Some(&reference.manifest.hash) {
                return Err("dormant installation unconfirmed".into());
            }
            if canic::method::call::<DormancyReceipt>(subject, 10, &())
                .await
                .map_err(|e| format!("dormancy receipt: {e:?}"))?
                != receipt
            {
                return Err("dormancy receipt differs".into());
            }
            P::Dormant
        }
        Some(current) => {
            if api::time() < deadline {
                return Err("original role proof window remains open".into());
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
                management(method, args, 0)
                    .await
                    .map_err(|e| format!("{method} issued: {e:?}"))?;
                return Ok(next);
            }
            if next == P::Returned {
                grant(subject)?;
                sharding::admit_pool_change(false)
                    .map_err(|e| format!("outstanding custody: {e:?}"))?;
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
