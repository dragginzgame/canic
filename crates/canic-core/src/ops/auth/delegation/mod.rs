//! Module: ops::auth::delegation
//!
//! Responsibility: manage issuer-local active delegation proof state and root
//! delegation proof batch metadata.
//! Does not own: endpoint authorization, IC call orchestration, or stable
//! auth record layout.
//! Boundary: API/workflow layers call this after endpoint guards have already
//! accepted the caller.
//!
//! Root proof provisioning shape:
//! - prepare runs in a root update and commits canister-signature leaves;
//! - get runs only as a direct root query so root has `data_certificate()`;
//! - install validates retrieved proofs against pending metadata before the
//!   runtime workflow broadcasts issuer-local installs.
//!
//! MVP invariant: pending batch metadata is bounded and pruned, while
//! signature-map leaves are retained.

#[cfg(test)]
mod tests;

use super::{
    AuthOps, PreparedRootDelegationProof,
    delegated::{
        active_proof::{
            InstallActiveDelegationProofError, InstallActiveDelegationProofInput,
            install_active_delegation_proof as build_active_delegation_proof,
        },
        canonical::cert_hash as delegation_cert_hash,
        cert_rules::DelegatedAuthTtlLimits,
        delegation_cert::{
            PrepareDelegationCertError, PrepareDelegationCertInput, finish_delegation_proof,
            prepare_delegation_cert,
        },
    },
    issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
    root_canister_sig::RootPayloadKind,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::auth::{
        AuthPolicyError, RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy,
        RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
        RootIssuerPolicy, validate_root_delegation_proof_prepare_policy,
    },
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse,
        AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience, DelegationProof,
        IssuerProofAlgorithm, IssuerProofBinding, RootDelegationProofBatchEntry,
        RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
        RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
        RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProof,
        RootDelegationProofInstallOutcome, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
        RootIssuerPolicyView, RootProof,
    },
    ops::{auth::AuthValidationError, ic::IcOps, storage::auth::AuthStateOps},
};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

const MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS: usize = 64;
const MAX_PENDING_ROOT_DELEGATION_PROOF_BATCHES: usize = 128;
const MAX_PENDING_ROOT_DELEGATION_PROOFS_PER_ISSUER: usize = 16;
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
struct PreparedRootDelegationProofBatchEntry {
    prepared: PreparedRootDelegationProof,
    installed: bool,
}

#[derive(Clone)]
struct PreparedRootDelegationProofBatchReplay {
    request_fingerprint: [u8; 32],
    response: RootDelegationProofBatchPrepareResponse,
    replay_expires_at_ns: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingDelegationProofBatchCleanup {
    pending_entries: usize,
    replay_entries: usize,
}

#[derive(Clone, Copy)]
struct RootDelegationProofBatchPrepareContext {
    metadata: AuthRequestMetadata,
    max_cert_ttl_ns: u64,
    issued_at_ns: u64,
}

// -----------------------------------------------------------------------------
// AuthOps Facade
// -----------------------------------------------------------------------------

impl AuthOps {
    pub(crate) fn install_active_delegation_proof(
        proof: DelegationProof,
        installed_by: Principal,
    ) -> Result<ActiveDelegationProof, InternalError> {
        let cfg = Self::auth_proof_verifier_config()?;
        let now_ns = IcOps::now_nanos();
        let active_proof = build_active_delegation_proof(
            InstallActiveDelegationProofInput {
                proof,
                installed_by,
                this_canister: IcOps::canister_self(),
                now_ns,
            },
            |cert_hash, root_proof, root_pid| {
                if root_pid != cfg.root_canister_id {
                    return Err(AuthValidationError::InvalidRootAuthority {
                        expected: cfg.root_canister_id,
                        found: root_pid,
                    }
                    .to_string());
                }
                Self::verify_root_canister_signature_proof(
                    RootPayloadKind::DelegationCert,
                    cert_hash,
                    root_proof,
                    cfg.root_canister_id,
                    &cfg.ic_root_public_key_raw,
                )
                .map_err(|err| err.to_string())
            },
        )
        .map_err(map_install_active_delegation_proof_error)?;

        Self::set_active_delegation_proof(active_proof.clone());
        Ok(active_proof)
    }

    #[must_use]
    pub(crate) fn active_delegation_proof(now_ns: u64) -> Option<ActiveDelegationProof> {
        AuthStateOps::active_delegation_proof(now_ns)
    }

    pub(crate) fn active_delegation_proof_status(
        now_ns: u64,
    ) -> ActiveDelegationProofStatusResponse {
        active_delegation_proof_status_response(
            now_ns,
            AuthStateOps::active_delegation_proof_snapshot(),
        )
    }

    pub(crate) fn set_active_delegation_proof(proof: ActiveDelegationProof) {
        AuthStateOps::set_active_delegation_proof(proof);
    }

    pub(crate) fn upsert_root_issuer_policy(
        request: RootIssuerPolicyUpsertRequest,
    ) -> Result<RootIssuerPolicyResponse, InternalError> {
        validate_root_issuer_policy_upsert_request(&request)?;

        let policy = root_issuer_policy_from_request(request);
        AuthStateOps::upsert_root_issuer_policy(policy.clone());

        Ok(RootIssuerPolicyResponse {
            issuer: root_issuer_policy_view(&policy),
        })
    }

    pub(crate) fn preflight_delegation_proof_batch_prepare_request(
        request: &RootDelegationProofBatchPrepareRequest,
        issued_at_ns: u64,
    ) -> Result<Vec<RootDelegationProofPreparePolicyDecision>, InternalError> {
        if request.entries.is_empty() {
            return Err(InternalError::invalid_input(
                "root delegation proof batch must contain at least one issuer",
            ));
        }
        ensure_root_delegation_proof_batch_entry_limit(request.entries.len())?;

        let mut decisions = Vec::with_capacity(request.entries.len());
        for entry in &request.entries {
            let audience = audience_policy(&entry.aud);
            let grants = grant_policies(&entry.grants);
            let policy = AuthStateOps::root_issuer_policy(entry.issuer_pid);

            let decision = validate_root_delegation_proof_prepare_policy(
                policy.as_ref(),
                RootDelegationProofPreparePolicyInput {
                    issuer_pid: entry.issuer_pid,
                    audience: &audience,
                    grants: &grants,
                    cert_ttl_ns: entry.cert_ttl_ns,
                    issued_at_ns,
                },
            )
            .map_err(map_root_provisioning_policy_error)?;
            decisions.push(decision);
        }

        Ok(decisions)
    }

    pub(crate) fn prepare_delegation_proof_batch(
        request: RootDelegationProofBatchPrepareRequest,
        max_cert_ttl_ns: u64,
        issued_at_ns: u64,
    ) -> Result<RootDelegationProofBatchPrepareResponse, InternalError> {
        let metadata = root_delegation_proof_batch_metadata(request.metadata)?;
        prepare_delegation_proof_batch_with_root_proof_replay(
            request,
            RootDelegationProofBatchPrepareContext {
                metadata,
                max_cert_ttl_ns,
                issued_at_ns,
            },
            |request| Self::preflight_delegation_proof_batch_prepare_request(request, issued_at_ns),
            IcOps::canister_self,
            |batch_id, cert_hash| {
                let root_pid = IcOps::canister_self();
                Self::prepare_root_canister_signature(
                    RootPayloadKind::DelegationCert,
                    batch_id,
                    cert_hash,
                    root_pid,
                    issued_at_ns,
                )
                .map(|prepared| prepared.retrieval_expires_at_ns)
            },
        )
    }

    pub(crate) fn get_delegation_proof_batch(
        request: RootDelegationProofBatchGetRequest,
    ) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
        let root_pid = IcOps::canister_self();
        let now_ns = IcOps::now_nanos();
        get_delegation_proof_batch_with_root_proof(request, root_pid, now_ns, |cert_hash| {
            Self::get_root_canister_signature_proof(
                RootPayloadKind::DelegationCert,
                cert_hash,
                root_pid,
                root_pid,
                now_ns,
            )
        })
    }

    pub(crate) fn preflight_delegation_proof_batch_install_proof(
        batch_id: [u8; 32],
        proof: &RootDelegationProofBatchProof,
        now_ns: u64,
    ) -> Result<(), RootDelegationProofInstallOutcome> {
        preflight_delegation_proof_batch_install_proof(batch_id, proof, now_ns)
    }

    pub(crate) fn mark_delegation_proof_batch_installed(
        batch_id: [u8; 32],
        issuer_pid: Principal,
        cert_hash: [u8; 32],
    ) {
        mark_delegation_proof_batch_installed(batch_id, issuer_pid, cert_hash);
    }

    pub(crate) fn prune_expired_delegation_proof_batch_metadata(now_ns: u64) {
        prune_expired_pending_delegation_proof_batch_metadata(now_ns);
    }
}

// -----------------------------------------------------------------------------
// Batch Preparation
// -----------------------------------------------------------------------------

fn prepare_delegation_proof_batch_with_root_proof_replay(
    request: RootDelegationProofBatchPrepareRequest,
    context: RootDelegationProofBatchPrepareContext,
    prepare_decisions: impl FnOnce(
        &RootDelegationProofBatchPrepareRequest,
    ) -> Result<
        Vec<RootDelegationProofPreparePolicyDecision>,
        InternalError,
    >,
    root_pid: impl FnOnce() -> Principal,
    prepare_root_proof: impl FnMut([u8; 32], [u8; 32]) -> Result<u64, InternalError>,
) -> Result<RootDelegationProofBatchPrepareResponse, InternalError> {
    prune_expired_pending_delegation_proof_batch_metadata(context.issued_at_ns);
    let batch_id = context.metadata.request_id;
    let request_fingerprint = root_delegation_proof_batch_prepare_request_fingerprint(&request);
    if let Some(response) = pending_delegation_proof_batch_replay_response(
        batch_id,
        request_fingerprint,
        context.issued_at_ns,
    )? {
        return Ok(response);
    }
    ensure_pending_delegation_proof_batch_quota(batch_id, &request.entries)?;

    let replay_expires_at_ns =
        root_delegation_proof_batch_replay_expires_at(context.metadata, context.issued_at_ns)?;
    let decisions = prepare_decisions(&request)?;
    let root_pid = root_pid();
    let response = prepare_delegation_proof_batch_with_root_proof(
        request,
        batch_id,
        decisions,
        context.max_cert_ttl_ns,
        context.issued_at_ns,
        root_pid,
        prepare_root_proof,
    )?;
    cache_prepared_delegation_proof_batch_replay(
        batch_id,
        request_fingerprint,
        response.clone(),
        replay_expires_at_ns,
    );
    Ok(response)
}

fn prepare_delegation_proof_batch_with_root_proof(
    request: RootDelegationProofBatchPrepareRequest,
    batch_id: [u8; 32],
    decisions: Vec<RootDelegationProofPreparePolicyDecision>,
    max_cert_ttl_ns: u64,
    issued_at_ns: u64,
    root_pid: Principal,
    mut prepare_root_proof: impl FnMut([u8; 32], [u8; 32]) -> Result<u64, InternalError>,
) -> Result<RootDelegationProofBatchPrepareResponse, InternalError> {
    let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
        seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
    };

    let mut response_entries = Vec::with_capacity(request.entries.len());
    let mut retrieval_expires_at_ns = u64::MAX;
    for (entry, decision) in request.entries.into_iter().zip(decisions) {
        let max_token_ttl_ns = entry.cert_ttl_ns.min(max_cert_ttl_ns);
        let prepared = prepare_delegation_cert(PrepareDelegationCertInput {
            root_pid,
            issuer_pid: entry.issuer_pid,
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding,
            issued_at_ns,
            cert_ttl_ns: entry.cert_ttl_ns,
            max_token_ttl_ns,
            audience: entry.aud,
            grants: entry.grants,
            ttl_limits: DelegatedAuthTtlLimits {
                max_cert_ttl_ns,
                max_token_ttl_ns,
            },
        })
        .map_err(map_prepare_delegation_cert_error)?;
        let entry_retrieval_expires_at_ns = prepare_root_proof(batch_id, prepared.cert_hash)?;
        retrieval_expires_at_ns = retrieval_expires_at_ns.min(entry_retrieval_expires_at_ns);

        let prepared = PreparedRootDelegationProof {
            cert: prepared.cert,
            cert_hash: prepared.cert_hash,
            retrieval_expires_at_ns: entry_retrieval_expires_at_ns,
        };
        cache_prepared_delegation_proof_batch(batch_id, entry.issuer_pid, prepared.clone());
        response_entries.push(RootDelegationProofBatchEntry {
            issuer_pid: entry.issuer_pid,
            cert_hash: prepared.cert_hash,
            expires_at_ns: decision.expires_at_ns,
            refresh_after_ns: decision.refresh_after_ns,
        });
    }

    Ok(RootDelegationProofBatchPrepareResponse {
        batch_id,
        entries: response_entries,
        retrieval_expires_at_ns,
    })
}

fn cache_prepared_delegation_proof_batch(
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

// -----------------------------------------------------------------------------
// Batch Retrieval
// -----------------------------------------------------------------------------

fn pending_delegation_proof_batch_entry(
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

fn get_delegation_proof_batch_with_root_proof(
    request: RootDelegationProofBatchGetRequest,
    root_pid: Principal,
    now_ns: u64,
    mut get_root_proof: impl FnMut([u8; 32]) -> Result<RootProof, InternalError>,
) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
    if request.entries.is_empty() {
        return Err(InternalError::invalid_input(
            "root delegation proof batch get must contain at least one proof reference",
        ));
    }

    let mut proofs = Vec::with_capacity(request.entries.len());
    for proof_ref in request.entries {
        let pending = pending_delegation_proof_batch_entry(
            request.batch_id,
            proof_ref.issuer_pid,
            proof_ref.cert_hash,
        )?;
        if pending.prepared.cert.issuer_pid != proof_ref.issuer_pid
            || pending.prepared.cert_hash != proof_ref.cert_hash
            || pending.prepared.cert.root_pid != root_pid
        {
            return Err(InternalError::invariant(
                crate::InternalErrorOrigin::Ops,
                "pending delegation proof batch metadata mismatch",
            ));
        }
        if now_ns >= pending.prepared.retrieval_expires_at_ns {
            return Err(AuthValidationError::Auth(
                "delegation proof batch retrieval window expired".to_string(),
            )
            .into());
        }

        let root_proof = get_root_proof(pending.prepared.cert_hash)?;
        let finalized = finish_delegation_proof(
            super::delegated::delegation_cert::PreparedDelegationCert {
                cert: pending.prepared.cert,
                cert_hash: pending.prepared.cert_hash,
            },
            root_proof,
        );
        proofs.push(RootDelegationProofBatchProof {
            issuer_pid: proof_ref.issuer_pid,
            cert_hash: finalized.cert_hash,
            proof: finalized.proof,
        });
    }

    Ok(RootDelegationProofBatchGetResponse {
        batch_id: request.batch_id,
        proofs,
    })
}

// -----------------------------------------------------------------------------
// Batch Installation And Cleanup
// -----------------------------------------------------------------------------

fn preflight_delegation_proof_batch_install_proof(
    batch_id: [u8; 32],
    proof: &RootDelegationProofBatchProof,
    now_ns: u64,
) -> Result<(), RootDelegationProofInstallOutcome> {
    if proof.proof.cert.issuer_pid != proof.issuer_pid {
        return Err(RootDelegationProofInstallOutcome::ProofMismatch);
    }
    if now_ns >= proof.proof.cert.expires_at_ns {
        return Err(RootDelegationProofInstallOutcome::ExpiredOrSuperseded);
    }
    let cert_hash = delegation_cert_hash(&proof.proof.cert)
        .map_err(|_| RootDelegationProofInstallOutcome::ProofMismatch)?;
    if cert_hash != proof.cert_hash {
        return Err(RootDelegationProofInstallOutcome::ProofMismatch);
    }

    let pending = pending_delegation_proof_batch_entry(batch_id, proof.issuer_pid, proof.cert_hash)
        .map_err(|_| RootDelegationProofInstallOutcome::ProofMismatch)?;
    if pending.prepared.cert_hash != proof.cert_hash
        || pending.prepared.cert != proof.proof.cert
        || pending.prepared.cert.issuer_pid != proof.issuer_pid
    {
        return Err(RootDelegationProofInstallOutcome::ProofMismatch);
    }
    if pending.installed {
        return Err(RootDelegationProofInstallOutcome::AlreadyInstalled);
    }
    if now_ns >= pending.prepared.retrieval_expires_at_ns
        || now_ns >= pending.prepared.cert.expires_at_ns
    {
        return Err(RootDelegationProofInstallOutcome::ExpiredOrSuperseded);
    }

    Ok(())
}

fn mark_delegation_proof_batch_installed(
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

fn prune_expired_pending_delegation_proof_batch_metadata(
    now_ns: u64,
) -> PendingDelegationProofBatchCleanup {
    PendingDelegationProofBatchCleanup {
        pending_entries: prune_expired_pending_delegation_proof_batches(now_ns),
        replay_entries: prune_expired_pending_delegation_proof_batch_replays(now_ns),
    }
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

// -----------------------------------------------------------------------------
// Replay Metadata
// -----------------------------------------------------------------------------

fn pending_delegation_proof_batch_replay_response(
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

fn cache_prepared_delegation_proof_batch_replay(
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

// -----------------------------------------------------------------------------
// Request Metadata And Fingerprinting
// -----------------------------------------------------------------------------

fn root_delegation_proof_batch_metadata(
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

fn root_delegation_proof_batch_replay_expires_at(
    metadata: AuthRequestMetadata,
    issued_at_ns: u64,
) -> Result<u64, InternalError> {
    issued_at_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
        InternalError::invalid_input(
            "root delegation proof batch replay metadata ttl_ns overflows expiry",
        )
    })
}

fn root_delegation_proof_batch_prepare_request_fingerprint(
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

// -----------------------------------------------------------------------------
// Quotas
// -----------------------------------------------------------------------------

fn ensure_root_delegation_proof_batch_entry_limit(entry_count: usize) -> Result<(), InternalError> {
    if entry_count > MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS {
        return Err(InternalError::resource_exhausted(format!(
            "root delegation proof batch issuer count {entry_count} exceeds max {MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS}",
        )));
    }
    Ok(())
}

fn ensure_pending_delegation_proof_batch_quota(
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

// -----------------------------------------------------------------------------
// Error Mapping
// -----------------------------------------------------------------------------

fn map_prepare_delegation_cert_error(err: PrepareDelegationCertError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError,
) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_root_provisioning_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::forbidden(err.to_string())
}

// -----------------------------------------------------------------------------
// Root Issuer Policy Mapping
// -----------------------------------------------------------------------------

fn validate_root_issuer_policy_upsert_request(
    request: &RootIssuerPolicyUpsertRequest,
) -> Result<(), InternalError> {
    if request.max_cert_ttl_ns == 0 {
        return Err(InternalError::invalid_input(
            "root issuer max certificate TTL must be greater than zero",
        ));
    }
    if request.refresh_after_ratio_bps == 0 || request.refresh_after_ratio_bps >= 10_000 {
        return Err(InternalError::invalid_input(
            "root issuer refresh ratio must be between 1 and 9999 basis points",
        ));
    }
    if request.enabled && request.allowed_audiences.is_empty() {
        return Err(InternalError::invalid_input(
            "enabled root issuer policy must allow at least one audience",
        ));
    }
    if request.enabled && request.allowed_grants.is_empty() {
        return Err(InternalError::invalid_input(
            "enabled root issuer policy must allow at least one grant",
        ));
    }
    Ok(())
}

fn root_issuer_policy_from_request(request: RootIssuerPolicyUpsertRequest) -> RootIssuerPolicy {
    RootIssuerPolicy {
        issuer_pid: request.issuer_pid,
        enabled: request.enabled,
        allowed_audiences: request
            .allowed_audiences
            .iter()
            .map(audience_policy)
            .collect(),
        allowed_grants: request.allowed_grants.iter().map(grant_policy).collect(),
        max_cert_ttl_ns: request.max_cert_ttl_ns,
        refresh_after_ratio_bps: request.refresh_after_ratio_bps,
    }
}

fn root_issuer_policy_view(policy: &RootIssuerPolicy) -> RootIssuerPolicyView {
    RootIssuerPolicyView {
        issuer_pid: policy.issuer_pid,
        enabled: policy.enabled,
        allowed_audiences: policy
            .allowed_audiences
            .iter()
            .map(delegation_audience_view)
            .collect(),
        allowed_grants: policy
            .allowed_grants
            .iter()
            .map(delegated_role_grant_view)
            .collect(),
        max_cert_ttl_ns: policy.max_cert_ttl_ns,
        refresh_after_ratio_bps: policy.refresh_after_ratio_bps,
    }
}

fn delegation_audience_view(policy: &RootDelegationAudiencePolicy) -> DelegationAudience {
    match policy {
        RootDelegationAudiencePolicy::Canister(canister) => DelegationAudience::Canister(*canister),
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            DelegationAudience::CanicSubnet(*subnet)
        }
        RootDelegationAudiencePolicy::Project(project) => {
            DelegationAudience::Project(project.clone())
        }
    }
}

fn delegated_role_grant_view(policy: &RootDelegatedRoleGrantPolicy) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: policy.target.clone(),
        scopes: policy.scopes.clone(),
    }
}

fn audience_policy(audience: &DelegationAudience) -> RootDelegationAudiencePolicy {
    match audience {
        DelegationAudience::Canister(canister) => RootDelegationAudiencePolicy::Canister(*canister),
        DelegationAudience::CanicSubnet(subnet) => {
            RootDelegationAudiencePolicy::CanicSubnet(*subnet)
        }
        DelegationAudience::Project(project) => {
            RootDelegationAudiencePolicy::Project(project.clone())
        }
    }
}

fn grant_policies(grants: &[DelegatedRoleGrant]) -> Vec<RootDelegatedRoleGrantPolicy> {
    grants.iter().map(grant_policy).collect()
}

fn grant_policy(grant: &DelegatedRoleGrant) -> RootDelegatedRoleGrantPolicy {
    RootDelegatedRoleGrantPolicy {
        target: grant.target.clone(),
        scopes: grant.scopes.clone(),
    }
}

// -----------------------------------------------------------------------------
// Active Proof Status
// -----------------------------------------------------------------------------

fn active_delegation_proof_status_response(
    now_ns: u64,
    proof: Option<ActiveDelegationProof>,
) -> ActiveDelegationProofStatusResponse {
    let Some(proof) = proof else {
        return ActiveDelegationProofStatusResponse {
            status: ActiveDelegationProofStatus::Missing,
            root_pid: None,
            issuer_pid: None,
            cert_hash: None,
            expires_at_ns: None,
            refresh_after_ns: None,
        };
    };

    let status = if now_ns >= proof.expires_at_ns {
        ActiveDelegationProofStatus::Expired
    } else if now_ns >= proof.refresh_after_ns {
        ActiveDelegationProofStatus::RefreshNeeded
    } else {
        ActiveDelegationProofStatus::Valid
    };

    ActiveDelegationProofStatusResponse {
        status,
        root_pid: Some(proof.proof.cert.root_pid),
        issuer_pid: Some(proof.proof.cert.issuer_pid),
        cert_hash: Some(proof.cert_hash),
        expires_at_ns: Some(proof.expires_at_ns),
        refresh_after_ns: Some(proof.refresh_after_ns),
    }
}
