//! Module: ops::auth::delegation::pending
//!
//! Responsibility: hold bounded in-memory root delegation proof batch metadata.
//! Does not own: policy decisions, proof assembly, or broadcast orchestration.

use super::super::PreparedRootDelegationProof;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience,
        RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
        RootDelegationProofBatchPrepareResponse,
    },
    ops::auth::AuthValidationError,
};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub(super) const MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS: usize = 64;
pub(super) const MAX_PENDING_ROOT_DELEGATION_PROOF_BATCHES: usize = 128;
pub(super) const MAX_PENDING_ROOT_DELEGATION_PROOFS_PER_ISSUER: usize = 16;
const MAX_ROOT_DELEGATION_PROOF_BATCH_REPLAY_TTL_NS: u64 = 60_000_000_000;
const ROOT_DELEGATION_PROOF_BATCH_PREPARE_FINGERPRINT_DOMAIN: &[u8] =
    b"canic-root-delegation-proof-batch-prepare-v1";

thread_local! {
    static PENDING_DELEGATION_PROOF_BATCHES: RefCell<BTreeMap<PendingDelegationProofBatchKey, PreparedRootDelegationProofBatchEntry>> =
        const { RefCell::new(BTreeMap::new()) };
    static PENDING_DELEGATION_PROOF_BATCH_REPLAYS: RefCell<BTreeMap<[u8; 32], PreparedRootDelegationProofBatchReplay>> =
        const { RefCell::new(BTreeMap::new()) };
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingDelegationProofBatchKey {
    batch_id: [u8; 32],
    issuer_pid: Vec<u8>,
    cert_hash: [u8; 32],
}

impl PendingDelegationProofBatchKey {
    fn new(batch_id: [u8; 32], issuer_pid: Principal, cert_hash: [u8; 32]) -> Self {
        Self {
            batch_id,
            issuer_pid: issuer_pid.as_slice().to_vec(),
            cert_hash,
        }
    }
}

#[derive(Clone)]
pub(super) struct PreparedRootDelegationProofBatchEntry {
    pub(super) prepared: PreparedRootDelegationProof,
    pub(super) installed: bool,
}

#[derive(Clone)]
struct PreparedRootDelegationProofBatchReplay {
    request_fingerprint: [u8; 32],
    response: RootDelegationProofBatchPrepareResponse,
    replay_expires_at_ns: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingDelegationProofBatchCleanup {
    pub(super) pending_entries: usize,
    pub(super) replay_entries: usize,
}

pub(super) fn cache_prepared_delegation_proof_batch(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    prepared: PreparedRootDelegationProof,
) {
    PENDING_DELEGATION_PROOF_BATCHES.with(|pending| {
        pending.borrow_mut().insert(
            PendingDelegationProofBatchKey::new(batch_id, issuer_pid, prepared.cert_hash),
            PreparedRootDelegationProofBatchEntry {
                prepared,
                installed: false,
            },
        );
    });
}

pub(super) fn pending_delegation_proof_batch_entry(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> Result<PreparedRootDelegationProofBatchEntry, InternalError> {
    PENDING_DELEGATION_PROOF_BATCHES
        .with(|pending| {
            pending
                .borrow()
                .get(&PendingDelegationProofBatchKey::new(
                    batch_id, issuer_pid, cert_hash,
                ))
                .cloned()
        })
        .ok_or_else(|| {
            AuthValidationError::Auth(
                "delegation proof batch entry was not prepared or has expired".to_string(),
            )
            .into()
        })
}

pub(super) fn mark_delegation_proof_batch_installed(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) {
    PENDING_DELEGATION_PROOF_BATCHES.with(|pending| {
        if let Some(entry) = pending
            .borrow_mut()
            .get_mut(&PendingDelegationProofBatchKey::new(
                batch_id, issuer_pid, cert_hash,
            ))
        {
            entry.installed = true;
        }
    });
}

pub(super) fn prune_expired_pending_delegation_proof_batch_metadata(
    now_ns: u64,
) -> PendingDelegationProofBatchCleanup {
    PendingDelegationProofBatchCleanup {
        pending_entries: prune_expired_pending_delegation_proof_batches(now_ns),
        replay_entries: prune_expired_pending_delegation_proof_batch_replays(now_ns),
    }
}

pub(super) fn pending_delegation_proof_batch_replay_response(
    batch_id: [u8; 32],
    request_fingerprint: [u8; 32],
    now_ns: u64,
) -> Result<Option<RootDelegationProofBatchPrepareResponse>, InternalError> {
    PENDING_DELEGATION_PROOF_BATCH_REPLAYS.with(|replays| {
        let mut replays = replays.borrow_mut();
        let Some(replay) = replays.get(&batch_id).cloned() else {
            return Ok(None);
        };
        if now_ns >= replay.replay_expires_at_ns {
            replays.remove(&batch_id);
            return Ok(None);
        }
        if replay.request_fingerprint != request_fingerprint {
            return Err(InternalError::invalid_input(
                "root delegation proof batch request_id was already used for a different request",
            ));
        }
        Ok(Some(replay.response))
    })
}

pub(super) fn cache_prepared_delegation_proof_batch_replay(
    batch_id: [u8; 32],
    request_fingerprint: [u8; 32],
    response: RootDelegationProofBatchPrepareResponse,
    replay_expires_at_ns: u64,
) {
    PENDING_DELEGATION_PROOF_BATCH_REPLAYS.with(|replays| {
        replays.borrow_mut().insert(
            batch_id,
            PreparedRootDelegationProofBatchReplay {
                request_fingerprint,
                response,
                replay_expires_at_ns,
            },
        );
    });
}

pub(super) fn root_delegation_proof_batch_metadata(
    metadata: Option<AuthRequestMetadata>,
) -> Result<AuthRequestMetadata, InternalError> {
    let metadata = metadata.ok_or_else(InternalError::operation_id_required)?;
    if metadata.ttl_ns == 0 {
        return Err(InternalError::invalid_input(
            "root delegation proof batch replay metadata ttl_ns must be greater than zero",
        ));
    }
    if metadata.ttl_ns > MAX_ROOT_DELEGATION_PROOF_BATCH_REPLAY_TTL_NS {
        return Err(InternalError::invalid_input(format!(
            "root delegation proof batch replay metadata ttl_ns={} exceeds max {}",
            metadata.ttl_ns, MAX_ROOT_DELEGATION_PROOF_BATCH_REPLAY_TTL_NS
        )));
    }
    Ok(metadata)
}

pub(super) fn root_delegation_proof_batch_replay_expires_at(
    metadata: AuthRequestMetadata,
    issued_at_ns: u64,
) -> Result<u64, InternalError> {
    issued_at_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
        InternalError::invalid_input(
            "root delegation proof batch replay metadata ttl_ns overflows expiry",
        )
    })
}

pub(super) fn root_delegation_proof_batch_prepare_request_fingerprint(
    request: &RootDelegationProofBatchPrepareRequest,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_prepare_bytes(
        &mut hasher,
        ROOT_DELEGATION_PROOF_BATCH_PREPARE_FINGERPRINT_DOMAIN,
    );
    hash_prepare_u64(&mut hasher, request.entries.len() as u64);
    for entry in &request.entries {
        hash_prepare_principal(&mut hasher, entry.issuer_pid);
        hash_prepare_audience(&mut hasher, &entry.aud);
        hash_prepare_grants(&mut hasher, &entry.grants);
        hash_prepare_u64(&mut hasher, entry.cert_ttl_ns);
    }
    hasher.finalize().into()
}

pub(super) fn ensure_root_delegation_proof_batch_entry_limit(
    entry_count: usize,
) -> Result<(), InternalError> {
    if entry_count > MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS {
        return Err(InternalError::resource_exhausted(format!(
            "root delegation proof batch issuer count {entry_count} exceeds max {MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS}",
        )));
    }
    Ok(())
}

pub(super) fn ensure_pending_delegation_proof_batch_quota(
    batch_id: [u8; 32],
    entries: &[RootDelegationProofBatchPrepareEntry],
) -> Result<(), InternalError> {
    ensure_root_delegation_proof_batch_entry_limit(entries.len())?;
    PENDING_DELEGATION_PROOF_BATCHES.with(|pending| {
        let pending = pending.borrow();
        let pending_batch_ids = pending
            .keys()
            .map(|key| key.batch_id)
            .collect::<BTreeSet<_>>();
        if !pending_batch_ids.contains(&batch_id)
            && pending_batch_ids.len() >= MAX_PENDING_ROOT_DELEGATION_PROOF_BATCHES
        {
            return Err(InternalError::resource_exhausted(format!(
                "root delegation proof pending batch count exceeds max {MAX_PENDING_ROOT_DELEGATION_PROOF_BATCHES}",
            )));
        }

        let mut requested_by_issuer: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        for entry in entries {
            *requested_by_issuer
                .entry(entry.issuer_pid.as_slice().to_vec())
                .or_default() += 1;
        }
        for (issuer_pid, requested_count) in requested_by_issuer {
            let existing_count = pending
                .keys()
                .filter(|key| key.issuer_pid == issuer_pid)
                .count();
            if existing_count.saturating_add(requested_count)
                > MAX_PENDING_ROOT_DELEGATION_PROOFS_PER_ISSUER
            {
                return Err(InternalError::resource_exhausted(format!(
                    "root delegation proof pending issuer proof count exceeds max {MAX_PENDING_ROOT_DELEGATION_PROOFS_PER_ISSUER}",
                )));
            }
        }

        Ok(())
    })
}

fn prune_expired_pending_delegation_proof_batches(now_ns: u64) -> usize {
    PENDING_DELEGATION_PROOF_BATCHES.with(|pending| {
        let mut pending = pending.borrow_mut();
        let before = pending.len();
        pending.retain(|_, entry| {
            if now_ns >= entry.prepared.cert.expires_at_ns {
                return false;
            }
            entry.installed || now_ns < entry.prepared.retrieval_expires_at_ns
        });
        before.saturating_sub(pending.len())
    })
}

fn prune_expired_pending_delegation_proof_batch_replays(now_ns: u64) -> usize {
    PENDING_DELEGATION_PROOF_BATCH_REPLAYS.with(|replays| {
        let mut replays = replays.borrow_mut();
        let before = replays.len();
        replays.retain(|_, replay| now_ns < replay.replay_expires_at_ns);
        before.saturating_sub(replays.len())
    })
}

fn hash_prepare_audience(hasher: &mut Sha256, audience: &DelegationAudience) {
    match audience {
        DelegationAudience::Canister(canister) => {
            hash_prepare_bytes(hasher, b"canister");
            hash_prepare_principal(hasher, *canister);
        }
        DelegationAudience::CanicSubnet(subnet) => {
            hash_prepare_bytes(hasher, b"canic_subnet");
            hash_prepare_principal(hasher, *subnet);
        }
        DelegationAudience::Project(project) => {
            hash_prepare_bytes(hasher, b"project");
            hash_prepare_bytes(hasher, project.as_bytes());
        }
    }
}

fn hash_prepare_grants(hasher: &mut Sha256, grants: &[DelegatedRoleGrant]) {
    hash_prepare_u64(hasher, grants.len() as u64);
    for grant in grants {
        hash_prepare_bytes(hasher, grant.target.as_str().as_bytes());
        hash_prepare_u64(hasher, grant.scopes.len() as u64);
        for scope in &grant.scopes {
            hash_prepare_bytes(hasher, scope.as_bytes());
        }
    }
}

fn hash_prepare_principal(hasher: &mut Sha256, principal: Principal) {
    hash_prepare_bytes(hasher, principal.as_slice());
}

fn hash_prepare_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn hash_prepare_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hash_prepare_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}
