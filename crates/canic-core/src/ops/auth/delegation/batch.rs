//! Module: ops::auth::delegation::batch
//!
//! Responsibility: prepare, retrieve, and install-check root delegation proof batches.
//! Does not own: pending metadata storage, issuer policy DTO mapping, or call broadcast.

use super::super::{
    AuthOps, PreparedRootDelegationProof,
    delegated::{
        canonical::cert_hash as delegation_cert_hash,
        cert_rules::DelegatedAuthTtlLimits,
        delegation_cert::{
            PrepareDelegationCertInput, finish_delegation_proof, prepare_delegation_cert,
        },
    },
    issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
    root_canister_sig::RootPayloadKind,
};
use super::{
    errors::{map_prepare_delegation_cert_error, map_root_provisioning_policy_error},
    pending::{
        cache_prepared_delegation_proof_batch, cache_prepared_delegation_proof_batch_replay,
        ensure_pending_delegation_proof_batch_quota,
        ensure_root_delegation_proof_batch_entry_limit, pending_delegation_proof_batch_entry,
        pending_delegation_proof_batch_replay_response, root_delegation_proof_batch_metadata,
        root_delegation_proof_batch_prepare_request_fingerprint,
        root_delegation_proof_batch_replay_expires_at,
    },
    root_issuer_policy::{audience_policy, grant_policies},
};
use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
        validate_root_delegation_proof_prepare_policy,
    },
    dto::auth::{
        AuthRequestMetadata, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchEntry, RootDelegationProofBatchGetRequest,
        RootDelegationProofBatchGetResponse, RootDelegationProofBatchPrepareRequest,
        RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProof,
        RootDelegationProofInstallOutcome, RootProof,
    },
    ops::{auth::AuthValidationError, ic::IcOps, storage::auth::AuthStateOps},
};

#[derive(Clone, Copy)]
pub(super) struct RootDelegationProofBatchPrepareContext {
    pub(super) metadata: AuthRequestMetadata,
    pub(super) max_cert_ttl_ns: u64,
    pub(super) issued_at_ns: u64,
}

pub(super) fn preflight_delegation_proof_batch_prepare_request(
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

pub(super) fn prepare_delegation_proof_batch(
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
        |request| preflight_delegation_proof_batch_prepare_request(request, issued_at_ns),
        IcOps::canister_self,
        |batch_id, cert_hash| {
            let root_pid = IcOps::canister_self();
            AuthOps::prepare_root_canister_signature(
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

pub(super) fn get_delegation_proof_batch(
    request: RootDelegationProofBatchGetRequest,
) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
    let root_pid = IcOps::canister_self();
    let now_ns = IcOps::now_nanos();
    get_delegation_proof_batch_with_root_proof(request, root_pid, now_ns, |cert_hash| {
        AuthOps::get_root_canister_signature_proof(
            RootPayloadKind::DelegationCert,
            cert_hash,
            root_pid,
            root_pid,
            now_ns,
        )
    })
}

pub(super) fn prepare_delegation_proof_batch_with_root_proof_replay(
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
    super::pending::prune_expired_pending_delegation_proof_batch_metadata(context.issued_at_ns);
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

pub(super) fn prepare_delegation_proof_batch_with_root_proof(
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

pub(super) fn get_delegation_proof_batch_with_root_proof(
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
            super::super::delegated::delegation_cert::PreparedDelegationCert {
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

pub(super) fn preflight_delegation_proof_batch_install_proof(
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
