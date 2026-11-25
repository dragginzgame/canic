//! Opt-in Root role issuer; its private seed never leaves the named lease.
use super::{STATE, api, sharding};
use canic::topology::attestation::RoleError;
use canic::topology::attestation::{Claim, Proof};
use canic_management_protocol::children::ChildId;
use ic_platform::ed25519::{Signer, SigningKey};
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableCell};
use ic_wire::{Bytes, Encoding, List};
use serde::{Deserialize, Serialize};

// Covers the seed and every shard slot at maximum principal and timestamp sizes.
const STORAGE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    nodes: 4096,
    allocation: 65536,
    depth: 16,
};

#[derive(Serialize, Deserialize)]
struct Issuer {
    seed: Option<[u8; 32]>,
    closed: List<(Principal, [u8; 32], u64), { ChildId::SHARDS.len() }>,
}

impl Issuer {
    fn deadline(&self, subject: Principal, generation: &[u8; 32]) -> Option<u64> {
        self.closed
            .as_slice()
            .iter()
            .find(|(p, g, _)| *p == subject && g == generation)
            .map(|entry| entry.2)
    }

    fn bytes(&self) -> Vec<u8> {
        ic_wire::encode(self, Encoding::Payload, STORAGE_LIMITS).expect("encode issuer")
    }

    fn load(cell: &StableCell<Vec<u8>>) -> Self {
        ic_wire::decode(cell.get(), Encoding::Payload, STORAGE_LIMITS).expect("open issuer")
    }
}

pub(crate) const KEY: &str = "manager.rolekey.v1";

pub(super) fn open(runtime: &MemoryRuntime<DefaultMemoryImpl>, fresh: bool) -> StableCell<Vec<u8>> {
    let cell = canic::storage::cell(
        runtime,
        KEY,
        STORAGE_LIMITS.bytes,
        fresh.then(|| {
            Issuer {
                seed: None,
                closed: List::new(vec![]).expect("empty closures"),
            }
            .bytes()
        }),
    );
    let _ = Issuer::load(&cell);
    cell
}

pub(super) fn validate_closures(
    state: &super::State,
    drained: impl Fn(Principal, &[u8; 32]) -> bool,
) {
    let issuer = Issuer::load(&state.role_key);
    for r in state.pools.retirements.values() {
        let target = r.target();
        assert_eq!(
            issuer.deadline(target.shard, &target.allocation.generation),
            Some(r.proof_deadline),
            "retirement must retain original issuer deadline"
        );
    }
    assert!(
        issuer
            .closed
            .as_slice()
            .iter()
            .all(|(p, g, _)| drained(*p, g)),
        "role closure without drained allocation"
    );
}

pub(super) fn require_unprepared() {
    STATE.with_borrow(|s| {
        assert!(
            Issuer::load(&s.as_ref().expect("open Root").role_key)
                .seed
                .is_none(),
            "role issuer custody required"
        )
    });
}

fn public(key: &[u8; 32]) -> Bytes<32> {
    Bytes::new(
        SigningKey::from_bytes(key)
            .verifying_key()
            .to_bytes()
            .to_vec(),
    )
    .expect("public key size")
}

pub(super) fn retained_public() -> Option<Bytes<32>> {
    STATE.with_borrow(|s| {
        Issuer::load(&s.as_ref().expect("open Root").role_key)
            .seed
            .as_ref()
            .map(public)
    })
}

/// Prepare the signing key once. Concurrent preparations converge on the first commit.
/// # Errors
/// Refuses unauthorized callers or unsuccessful entropy acquisition.
/// # Panics
/// Requires an open, intact Root key lease.
pub async fn prepare() -> Result<Bytes<32>, RoleError> {
    let controller = api::caller();
    if !api::is_controller(&controller) {
        return Err(RoleError::Unauthorized);
    }
    if let Some(key) = retained_public() {
        return Ok(key);
    }
    let fresh = canic::entropy::fresh(5)
        .await
        .map_err(|_| RoleError::EntropyUnavailable)?;
    if !api::is_controller(&controller) {
        return Err(RoleError::Unauthorized);
    }
    STATE.with_borrow_mut(|s| {
        let cell = &mut s.as_mut().expect("open Root").role_key;
        let mut issuer = Issuer::load(cell);
        if let Some(key) = &issuer.seed {
            return Ok(public(key));
        }
        issuer.seed = Some(fresh);
        cell.set(issuer.bytes());
        Ok(public(&fresh))
    })
}

/// Sign the ready caller’s original installation role for one receiver.
/// # Errors
/// Requires an initialized signer, a canister audience and a verified role-bearing installation.
/// # Panics
/// Requires an open, intact Root key lease.
pub fn issue(receiver: Principal) -> Result<Proof, RoleError> {
    if receiver.as_slice().last() != Some(&1) {
        return Err(RoleError::InvalidReceiver);
    }
    let assignment = sharding::assigned_role(api::caller())?;
    let generation = assignment.generation;
    let claim = Claim {
        root: api::canister_self(),
        subject: api::caller(),
        receiver,
        role: assignment.role,
        generation: Bytes::new(generation.to_vec()).expect("generation size"),
        issued_at_ns: api::time(),
    };
    claim.expiry().map_err(|_| RoleError::InvalidClaim)?;
    let message = claim.signing_bytes().map_err(|_| RoleError::InvalidClaim)?;
    STATE.with_borrow(|s| {
        let issuer = Issuer::load(&s.as_ref().expect("open Root").role_key);
        if issuer.deadline(claim.subject, &generation).is_some() {
            return Err(RoleError::Closed);
        }
        let seed = issuer.seed.as_ref().ok_or(RoleError::KeyUnavailable)?;
        let signature = SigningKey::from_bytes(seed).sign(&message);
        Ok((
            claim,
            Bytes::new(signature.to_bytes().to_vec()).expect("signature size"),
        ))
    })
}

/// Fence renewal for an exact ready assignment and return the original safe reuse time.
/// # Errors
/// Refuses unauthorized or stale requests and exhausted retirement capacity.
/// # Panics
/// Requires an open Root issuer lease.
pub fn close(subject: Principal, generation: &Bytes<32>) -> Result<u64, RoleError> {
    if !api::is_controller(&api::caller()) {
        return Err(RoleError::Unauthorized);
    }
    let generation: [u8; 32] = generation
        .as_slice()
        .try_into()
        .map_err(|_| RoleError::InvalidGeneration)?;
    if let Some(deadline) = deadline(subject, &generation) {
        return Ok(deadline);
    }
    let assignment = sharding::assigned_role(subject)?;
    let evacuated = assignment.evacuated_shard.ok_or(RoleError::NotShard)?;
    if assignment.generation != generation {
        return Err(RoleError::GenerationChanged);
    }
    if !evacuated {
        return Err(RoleError::NotDrained);
    }
    let deadline = api::time()
        .checked_add(canic::topology::attestation::LIFETIME_NS)
        .ok_or(RoleError::InvalidClaim)?;
    STATE.with_borrow_mut(|s| {
        let cell = &mut s.as_mut().expect("open Root").role_key;
        let mut issuer = Issuer::load(cell);
        if issuer.seed.is_none() {
            return Err(RoleError::KeyUnavailable);
        }
        let mut entries = issuer.closed.into_vec();
        entries.push((subject, generation, deadline));
        issuer.closed = List::new(entries).map_err(|_| RoleError::Capacity)?;
        cell.set(issuer.bytes());
        Ok(deadline)
    })
}

#[cfg(test)]
mod tests;

pub(super) fn deadline(subject: Principal, generation: &[u8; 32]) -> Option<u64> {
    STATE.with_borrow(|s| {
        Issuer::load(&s.as_ref().expect("open Root").role_key).deadline(subject, generation)
    })
}

/// Observe the proof deadline and live placement count for a closed generation.
/// Neither observation authorizes a wipe or a handover.
/// # Errors
/// Refuses an open issuer or a stale generation.
pub fn retirement_status(
    subject: Principal,
    generation: &Bytes<32>,
) -> Result<(u64, u32), RoleError> {
    let generation: [u8; 32] = generation
        .as_slice()
        .try_into()
        .map_err(|_| RoleError::InvalidGeneration)?;
    if let Some(deadline) = deadline(subject, &generation) {
        // A closure is admitted only at zero and the draining generation cannot grow.
        return Ok((deadline, 0));
    }
    let assignment = sharding::assigned_role(subject)?;
    assignment.evacuated_shard.ok_or(RoleError::NotShard)?;
    let actual = assignment.generation;
    if generation != actual {
        return Err(RoleError::GenerationChanged);
    }
    Err(RoleError::Open)
}
