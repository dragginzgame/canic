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
        validate_root_delegation_proof_prepare_policy,
    },
    dto::{
        auth::{
            ActiveDelegationProof, ActiveDelegationProofStatus,
            ActiveDelegationProofStatusResponse, AuthRequestMetadata, DelegatedRoleGrant,
            DelegationAudience, DelegationProof, IssuerProofAlgorithm, IssuerProofBinding,
            RootDelegationProofBatchEntry, RootDelegationProofBatchGetRequest,
            RootDelegationProofBatchGetResponse, RootDelegationProofBatchPrepareRequest,
            RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProof,
            RootDelegationProofInstallOutcome, RootProof,
        },
        error::Error,
    },
    ops::{auth::AuthValidationError, ic::IcOps, storage::auth::AuthStateOps},
};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap};

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

#[derive(Clone, Copy)]
struct RootDelegationProofBatchPrepareContext {
    metadata: AuthRequestMetadata,
    max_cert_ttl_ns: u64,
    issued_at_ns: u64,
}

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

    pub(crate) fn preflight_delegation_proof_batch_prepare_request(
        request: &RootDelegationProofBatchPrepareRequest,
        issued_at_ns: u64,
    ) -> Result<Vec<RootDelegationProofPreparePolicyDecision>, InternalError> {
        if request.entries.is_empty() {
            return Err(InternalError::public(Error::invalid(
                "root delegation proof batch must contain at least one issuer",
            )));
        }

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
        prepare_delegation_proof_batch_with_root_signature_replay(
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
        get_delegation_proof_batch_with_root_signature(request, root_pid, now_ns, |cert_hash| {
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
}

fn prepare_delegation_proof_batch_with_root_signature_replay(
    request: RootDelegationProofBatchPrepareRequest,
    context: RootDelegationProofBatchPrepareContext,
    prepare_decisions: impl FnOnce(
        &RootDelegationProofBatchPrepareRequest,
    ) -> Result<
        Vec<RootDelegationProofPreparePolicyDecision>,
        InternalError,
    >,
    root_pid: impl FnOnce() -> Principal,
    prepare_signature: impl FnMut([u8; 32], [u8; 32]) -> Result<u64, InternalError>,
) -> Result<RootDelegationProofBatchPrepareResponse, InternalError> {
    let batch_id = context.metadata.request_id;
    let request_fingerprint = root_delegation_proof_batch_prepare_request_fingerprint(&request);
    if let Some(response) = pending_delegation_proof_batch_replay_response(
        batch_id,
        request_fingerprint,
        context.issued_at_ns,
    )? {
        return Ok(response);
    }

    let replay_expires_at_ns =
        root_delegation_proof_batch_replay_expires_at(context.metadata, context.issued_at_ns)?;
    let decisions = prepare_decisions(&request)?;
    let root_pid = root_pid();
    let response = prepare_delegation_proof_batch_with_root_signature(
        request,
        batch_id,
        decisions,
        context.max_cert_ttl_ns,
        context.issued_at_ns,
        root_pid,
        prepare_signature,
    )?;
    cache_prepared_delegation_proof_batch_replay(
        batch_id,
        request_fingerprint,
        response.clone(),
        replay_expires_at_ns,
    );
    Ok(response)
}

fn prepare_delegation_proof_batch_with_root_signature(
    request: RootDelegationProofBatchPrepareRequest,
    batch_id: [u8; 32],
    decisions: Vec<RootDelegationProofPreparePolicyDecision>,
    max_cert_ttl_ns: u64,
    issued_at_ns: u64,
    root_pid: Principal,
    mut prepare_signature: impl FnMut([u8; 32], [u8; 32]) -> Result<u64, InternalError>,
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
        let entry_retrieval_expires_at_ns = prepare_signature(batch_id, prepared.cert_hash)?;
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

fn get_delegation_proof_batch_with_root_signature(
    request: RootDelegationProofBatchGetRequest,
    root_pid: Principal,
    now_ns: u64,
    mut get_root_proof: impl FnMut([u8; 32]) -> Result<RootProof, InternalError>,
) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
    if request.entries.is_empty() {
        return Err(InternalError::public(Error::invalid(
            "root delegation proof batch get must contain at least one proof reference",
        )));
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
            return Err(InternalError::public(Error::invalid(
                "root delegation proof batch request_id was already used for a different request",
            )));
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

fn root_delegation_proof_batch_metadata(
    metadata: Option<AuthRequestMetadata>,
) -> Result<AuthRequestMetadata, InternalError> {
    let metadata = metadata.ok_or_else(|| InternalError::public(Error::operation_id_required()))?;
    if metadata.ttl_ns == 0 {
        return Err(InternalError::public(Error::invalid(
            "root delegation proof batch replay metadata ttl_ns must be greater than zero",
        )));
    }
    if metadata.ttl_ns > MAX_ROOT_DELEGATION_PROOF_BATCH_REPLAY_TTL_NS {
        return Err(InternalError::public(Error::invalid(format!(
            "root delegation proof batch replay metadata ttl_ns={} exceeds max {}",
            metadata.ttl_ns, MAX_ROOT_DELEGATION_PROOF_BATCH_REPLAY_TTL_NS
        ))));
    }
    Ok(metadata)
}

fn root_delegation_proof_batch_replay_expires_at(
    metadata: AuthRequestMetadata,
    issued_at_ns: u64,
) -> Result<u64, InternalError> {
    issued_at_ns.checked_add(metadata.ttl_ns).ok_or_else(|| {
        InternalError::public(Error::invalid(
            "root delegation proof batch replay metadata ttl_ns overflows expiry",
        ))
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

fn map_prepare_delegation_cert_error(err: PrepareDelegationCertError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError,
) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_root_provisioning_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::public(Error::forbidden(err.to_string()))
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
    grants
        .iter()
        .map(|grant| RootDelegatedRoleGrantPolicy {
            target: grant.target.clone(),
            scopes: grant.scopes.clone(),
        })
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InternalErrorClass,
        domain::policy::auth::RootIssuerPolicy,
        dto::auth::{
            AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience, DelegationCert,
            IcCanisterSignatureProofV1, RootDelegationProofBatchGetRequest,
            RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
            RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProofRef, RootProof,
        },
        dto::error::ErrorCode,
        ids::{CanisterRole, cap},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn active_proof() -> ActiveDelegationProof {
        ActiveDelegationProof {
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(1),
                    issuer_pid: p(2),
                    issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                    issuer_proof_binding_hash: [3; 32],
                    issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                        seed_hash: [4; 32],
                    },
                    issued_at_ns: 10,
                    not_before_ns: 20,
                    expires_at_ns: 100,
                    max_token_ttl_ns: 30,
                    aud: DelegationAudience::CanicSubnet(p(7)),
                    grants: vec![DelegatedRoleGrant {
                        target: CanisterRole::owned("project_instance".to_string()),
                        scopes: vec!["canic.issue".to_string()],
                    }],
                },
                root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                    signature_cbor: vec![8; 64],
                    public_key_der: vec![9; 32],
                }),
            },
            cert_hash: [10; 32],
            not_before_ns: 20,
            expires_at_ns: 100,
            refresh_after_ns: 80,
            installed_at_ns: 20,
            installed_by: p(11),
        }
    }

    fn batch_prepare_entry(
        issuer_pid: Principal,
        cert_ttl_ns: u64,
    ) -> RootDelegationProofBatchPrepareEntry {
        RootDelegationProofBatchPrepareEntry {
            issuer_pid,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec![cap::READ.to_string()],
            }],
            cert_ttl_ns,
        }
    }

    fn batch_prepare_request(
        issuer_pid: Principal,
        cert_ttl_ns: u64,
    ) -> RootDelegationProofBatchPrepareRequest {
        RootDelegationProofBatchPrepareRequest {
            metadata: None,
            entries: vec![batch_prepare_entry(issuer_pid, cert_ttl_ns)],
        }
    }

    fn metadata(id: u8, ttl_ns: u64) -> AuthRequestMetadata {
        AuthRequestMetadata {
            request_id: [id; 32],
            ttl_ns,
        }
    }

    fn root_issuer_policy(issuer_pid: Principal) -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![RootDelegationAudiencePolicy::Project("test".to_string())],
            allowed_grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec![cap::READ.to_string()],
            }],
            max_cert_ttl_ns: 120_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn root_proof(byte: u8) -> RootProof {
        RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![byte; 8],
            public_key_der: vec![byte; 4],
        })
    }

    fn prepared_batch(
        issuer_pid: Principal,
        metadata_id: u8,
        retrieval_expires_at_ns: u64,
    ) -> RootDelegationProofBatchPrepareResponse {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(issuer_pid));
        let mut request = batch_prepare_request(issuer_pid, 60_000_000_000);
        request.metadata = Some(metadata(metadata_id, 60_000_000_000));
        let decisions =
            AuthOps::preflight_delegation_proof_batch_prepare_request(&request, 10).unwrap();

        prepare_delegation_proof_batch_with_root_signature(
            request,
            [metadata_id; 32],
            decisions,
            120_000_000_000,
            10,
            p(1),
            |batch_id, _cert_hash| {
                assert_eq!(batch_id, [metadata_id; 32]);
                Ok(retrieval_expires_at_ns)
            },
        )
        .expect("batch prepare should produce metadata")
    }

    fn batch_get_request(
        response: &RootDelegationProofBatchPrepareResponse,
    ) -> RootDelegationProofBatchGetRequest {
        RootDelegationProofBatchGetRequest {
            batch_id: response.batch_id,
            entries: response
                .entries
                .iter()
                .map(|entry| RootDelegationProofBatchProofRef {
                    issuer_pid: entry.issuer_pid,
                    cert_hash: entry.cert_hash,
                })
                .collect(),
        }
    }

    #[test]
    fn active_delegation_proof_status_reports_missing() {
        let status = active_delegation_proof_status_response(50, None);

        assert_eq!(status.status, ActiveDelegationProofStatus::Missing);
        assert_eq!(status.root_pid, None);
        assert_eq!(status.cert_hash, None);
    }

    #[test]
    fn active_delegation_proof_status_reports_lifecycle_states() {
        let valid = active_delegation_proof_status_response(79, Some(active_proof()));
        assert_eq!(valid.status, ActiveDelegationProofStatus::Valid);
        assert_eq!(valid.root_pid, Some(p(1)));
        assert_eq!(valid.issuer_pid, Some(p(2)));
        assert_eq!(valid.cert_hash, Some([10; 32]));
        assert_eq!(valid.expires_at_ns, Some(100));
        assert_eq!(valid.refresh_after_ns, Some(80));

        let refresh = active_delegation_proof_status_response(80, Some(active_proof()));
        assert_eq!(refresh.status, ActiveDelegationProofStatus::RefreshNeeded);

        let expired = active_delegation_proof_status_response(100, Some(active_proof()));
        assert_eq!(expired.status, ActiveDelegationProofStatus::Expired);
    }

    #[test]
    fn batch_prepare_preflight_accepts_registered_issuer_policy() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(21)));

        let decisions = AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(p(21), 60_000_000_000),
            10,
        )
        .expect("valid batch prepare shape");

        assert_eq!(
            decisions,
            vec![RootDelegationProofPreparePolicyDecision {
                expires_at_ns: 60_000_000_010,
                refresh_after_ns: 48_000_000_010,
            }]
        );
    }

    #[test]
    fn batch_prepare_preflight_rejects_ttl_above_max() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(22)));

        let err = AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(p(22), 121_000_000_000),
            10,
        )
        .expect_err("ttl above max must fail preflight");
        let public = err.public_error().expect("public policy error");

        assert_eq!(public.code, ErrorCode::Forbidden);
    }

    #[test]
    fn batch_prepare_preflight_rejects_unregistered_issuer() {
        let err = AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(p(23), 60_000_000_000),
            10,
        )
        .expect_err("unregistered issuer must fail preflight");
        let public = err.public_error().expect("public policy error");

        assert_eq!(public.code, ErrorCode::Forbidden);
    }

    #[test]
    fn batch_prepare_preflight_rejects_disabled_issuer() {
        let mut policy = root_issuer_policy(p(24));
        policy.enabled = false;
        AuthStateOps::upsert_root_issuer_policy(policy);

        let err = AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(p(24), 60_000_000_000),
            10,
        )
        .expect_err("disabled issuer must fail preflight");
        let public = err.public_error().expect("public policy error");

        assert_eq!(public.code, ErrorCode::Forbidden);
    }

    #[test]
    fn batch_prepare_preflight_rejects_grant_outside_issuer_policy() {
        let mut policy = root_issuer_policy(p(25));
        policy.allowed_grants[0].scopes = vec!["canic.read".to_string()];
        AuthStateOps::upsert_root_issuer_policy(policy);

        let err = AuthOps::preflight_delegation_proof_batch_prepare_request(
            &batch_prepare_request(p(25), 60_000_000_000),
            10,
        )
        .expect_err("grant outside issuer policy must fail preflight");
        let public = err.public_error().expect("public policy error");

        assert_eq!(public.code, ErrorCode::Forbidden);
    }

    #[test]
    fn batch_prepare_rejects_missing_metadata() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(26)));

        let err = AuthOps::prepare_delegation_proof_batch(
            batch_prepare_request(p(26), 60_000_000_000),
            120_000_000_000,
            10,
        )
        .expect_err("batch prepare requires request id metadata");
        let public = err.public_error().expect("public metadata error");

        assert_eq!(public.code, ErrorCode::OperationIdRequired);
    }

    #[test]
    fn batch_prepare_rejects_empty_entries() {
        let request = RootDelegationProofBatchPrepareRequest {
            metadata: Some(metadata(27, 60_000_000_000)),
            entries: vec![],
        };

        let err = AuthOps::prepare_delegation_proof_batch(request, 120_000_000_000, 10)
            .expect_err("empty batch must fail");
        let public = err.public_error().expect("public batch error");

        assert_eq!(public.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn batch_prepare_rejects_invalid_metadata_ttl() {
        let request = RootDelegationProofBatchPrepareRequest {
            metadata: Some(metadata(28, 0)),
            entries: vec![batch_prepare_entry(p(28), 60_000_000_000)],
        };

        let err = AuthOps::prepare_delegation_proof_batch(request, 120_000_000_000, 10)
            .expect_err("zero metadata ttl must fail");
        let public = err.public_error().expect("public metadata error");

        assert_eq!(public.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn batch_prepare_returns_metadata_for_registered_issuer() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(29)));
        let mut request = batch_prepare_request(p(29), 60_000_000_000);
        request.metadata = Some(metadata(29, 60_000_000_000));
        let decisions =
            AuthOps::preflight_delegation_proof_batch_prepare_request(&request, 10).unwrap();
        let mut signed_hashes = Vec::new();

        let response = prepare_delegation_proof_batch_with_root_signature(
            request,
            [29; 32],
            decisions,
            120_000_000_000,
            10,
            p(1),
            |batch_id, cert_hash| {
                assert_eq!(batch_id, [29; 32]);
                signed_hashes.push(cert_hash);
                Ok(70)
            },
        )
        .expect("batch prepare should produce metadata");

        assert_eq!(response.batch_id, [29; 32]);
        assert_eq!(response.entries.len(), 1);
        assert_eq!(response.entries[0].issuer_pid, p(29));
        assert_eq!(response.entries[0].expires_at_ns, 60_000_000_010);
        assert_eq!(response.entries[0].refresh_after_ns, 48_000_000_010);
        assert_eq!(response.retrieval_expires_at_ns, 70);
        assert_eq!(signed_hashes, vec![response.entries[0].cert_hash]);
    }

    #[test]
    fn batch_prepare_replays_same_request_id_without_resigning() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(30)));
        let mut request = batch_prepare_request(p(30), 60_000_000_000);
        let metadata = metadata(30, 60_000_000_000);
        request.metadata = Some(metadata);
        let context = RootDelegationProofBatchPrepareContext {
            metadata,
            max_cert_ttl_ns: 120_000_000_000,
            issued_at_ns: 10,
        };
        let mut sign_count = 0;

        let first = prepare_delegation_proof_batch_with_root_signature_replay(
            request.clone(),
            context,
            |request| AuthOps::preflight_delegation_proof_batch_prepare_request(request, 10),
            || p(1),
            |batch_id, _cert_hash| {
                assert_eq!(batch_id, [30; 32]);
                sign_count += 1;
                Ok(70)
            },
        )
        .expect("first batch prepare should produce metadata");

        let replay = prepare_delegation_proof_batch_with_root_signature_replay(
            request,
            RootDelegationProofBatchPrepareContext {
                issued_at_ns: 20,
                ..context
            },
            |_request| -> Result<Vec<RootDelegationProofPreparePolicyDecision>, InternalError> {
                panic!("cached replay must not rerun policy preflight")
            },
            || p(1),
            |_batch_id, _cert_hash| -> Result<u64, InternalError> {
                panic!("cached replay must not prepare a new signature")
            },
        )
        .expect("same request id and payload should replay original metadata");

        assert_eq!(replay, first);
        assert_eq!(sign_count, 1);
    }

    #[test]
    fn batch_prepare_rejects_conflicting_request_id_reuse() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(31)));
        let mut request = batch_prepare_request(p(31), 60_000_000_000);
        let metadata = metadata(31, 60_000_000_000);
        request.metadata = Some(metadata);
        let context = RootDelegationProofBatchPrepareContext {
            metadata,
            max_cert_ttl_ns: 120_000_000_000,
            issued_at_ns: 10,
        };

        prepare_delegation_proof_batch_with_root_signature_replay(
            request.clone(),
            context,
            |request| AuthOps::preflight_delegation_proof_batch_prepare_request(request, 10),
            || p(1),
            |_batch_id, _cert_hash| Ok(70),
        )
        .expect("first batch prepare should produce metadata");

        let mut conflicting = request;
        conflicting.entries[0].cert_ttl_ns = 30_000_000_000;
        let err = prepare_delegation_proof_batch_with_root_signature_replay(
            conflicting,
            RootDelegationProofBatchPrepareContext {
                issued_at_ns: 20,
                ..context
            },
            |_request| -> Result<Vec<RootDelegationProofPreparePolicyDecision>, InternalError> {
                panic!("conflicting replay must fail before policy preflight")
            },
            || p(1),
            |_batch_id, _cert_hash| -> Result<u64, InternalError> {
                panic!("conflicting replay must fail before signature preparation")
            },
        )
        .expect_err("request id reuse with a different payload must fail");
        let public = err.public_error().expect("public replay conflict error");

        assert_eq!(public.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn batch_get_rejects_empty_entries() {
        let err = get_delegation_proof_batch_with_root_signature(
            RootDelegationProofBatchGetRequest {
                batch_id: [40; 32],
                entries: vec![],
            },
            p(1),
            10,
            |_cert_hash| panic!("empty get request must not request a root proof"),
        )
        .expect_err("empty get request must fail");
        let public = err.public_error().expect("public get error");

        assert_eq!(public.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn batch_get_rejects_missing_pending_metadata() {
        let err = get_delegation_proof_batch_with_root_signature(
            RootDelegationProofBatchGetRequest {
                batch_id: [41; 32],
                entries: vec![RootDelegationProofBatchProofRef {
                    issuer_pid: p(41),
                    cert_hash: [1; 32],
                }],
            },
            p(1),
            10,
            |_cert_hash| panic!("missing pending entry must not request a root proof"),
        )
        .expect_err("missing pending entry must fail");

        assert_eq!(err.class(), InternalErrorClass::Ops);
    }

    #[test]
    fn batch_get_rejects_expired_pending_metadata() {
        let response = prepared_batch(p(42), 42, 70);

        let err = get_delegation_proof_batch_with_root_signature(
            batch_get_request(&response),
            p(1),
            70,
            |_cert_hash| panic!("expired pending entry must not request a root proof"),
        )
        .expect_err("expired pending entry must fail");

        assert_eq!(err.class(), InternalErrorClass::Ops);
    }

    #[test]
    fn batch_get_returns_prepared_proofs() {
        let response = prepared_batch(p(43), 43, 90);
        let mut requested_hashes = Vec::new();

        let retrieved = get_delegation_proof_batch_with_root_signature(
            batch_get_request(&response),
            p(1),
            80,
            |cert_hash| {
                requested_hashes.push(cert_hash);
                Ok(root_proof(9))
            },
        )
        .expect("prepared batch should retrieve proofs");

        assert_eq!(retrieved.batch_id, response.batch_id);
        assert_eq!(retrieved.proofs.len(), 1);
        assert_eq!(retrieved.proofs[0].issuer_pid, p(43));
        assert_eq!(retrieved.proofs[0].cert_hash, response.entries[0].cert_hash);
        assert_eq!(retrieved.proofs[0].proof.cert.root_pid, p(1));
        assert_eq!(retrieved.proofs[0].proof.cert.issuer_pid, p(43));
        assert_eq!(requested_hashes, vec![response.entries[0].cert_hash]);
    }

    #[test]
    fn batch_get_preserves_requested_entry_order() {
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(44)));
        AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(45)));
        let request = RootDelegationProofBatchPrepareRequest {
            metadata: Some(metadata(44, 60_000_000_000)),
            entries: vec![
                batch_prepare_entry(p(44), 60_000_000_000),
                batch_prepare_entry(p(45), 60_000_000_000),
            ],
        };
        let decisions =
            AuthOps::preflight_delegation_proof_batch_prepare_request(&request, 10).unwrap();
        let response = prepare_delegation_proof_batch_with_root_signature(
            request,
            [44; 32],
            decisions,
            120_000_000_000,
            10,
            p(1),
            |_batch_id, _cert_hash| Ok(90),
        )
        .expect("batch prepare should produce metadata");
        let requested_entries = response
            .entries
            .iter()
            .rev()
            .map(|entry| RootDelegationProofBatchProofRef {
                issuer_pid: entry.issuer_pid,
                cert_hash: entry.cert_hash,
            })
            .collect::<Vec<_>>();
        let expected_hashes = requested_entries
            .iter()
            .map(|entry| entry.cert_hash)
            .collect::<Vec<_>>();
        let expected_issuers = requested_entries
            .iter()
            .map(|entry| entry.issuer_pid)
            .collect::<Vec<_>>();
        let mut requested_hashes = Vec::new();

        let retrieved = get_delegation_proof_batch_with_root_signature(
            RootDelegationProofBatchGetRequest {
                batch_id: response.batch_id,
                entries: requested_entries,
            },
            p(1),
            80,
            |cert_hash| {
                requested_hashes.push(cert_hash);
                Ok(root_proof(10))
            },
        )
        .expect("prepared batch should retrieve proofs");

        let retrieved_hashes = retrieved
            .proofs
            .iter()
            .map(|proof| proof.cert_hash)
            .collect::<Vec<_>>();
        let retrieved_issuers = retrieved
            .proofs
            .iter()
            .map(|proof| proof.issuer_pid)
            .collect::<Vec<_>>();
        assert_eq!(retrieved_hashes, expected_hashes);
        assert_eq!(retrieved_issuers, expected_issuers);
        assert_eq!(requested_hashes, expected_hashes);
    }

    #[test]
    fn batch_install_preflight_accepts_retrieved_proof() {
        let response = prepared_batch(p(46), 46, 90);
        let retrieved = get_delegation_proof_batch_with_root_signature(
            batch_get_request(&response),
            p(1),
            80,
            |_cert_hash| Ok(root_proof(11)),
        )
        .expect("prepared batch should retrieve proofs");

        assert_eq!(
            preflight_delegation_proof_batch_install_proof(
                response.batch_id,
                &retrieved.proofs[0],
                80,
            ),
            Ok(())
        );
    }

    #[test]
    fn batch_install_preflight_rejects_proof_mismatch() {
        let response = prepared_batch(p(47), 47, 90);
        let retrieved = get_delegation_proof_batch_with_root_signature(
            batch_get_request(&response),
            p(1),
            80,
            |_cert_hash| Ok(root_proof(12)),
        )
        .expect("prepared batch should retrieve proofs");
        let mut proof = retrieved.proofs[0].clone();
        proof.cert_hash = [9; 32];

        assert_eq!(
            preflight_delegation_proof_batch_install_proof(response.batch_id, &proof, 80),
            Err(RootDelegationProofInstallOutcome::ProofMismatch)
        );
    }

    #[test]
    fn batch_install_preflight_rejects_stale_pending_metadata() {
        let response = prepared_batch(p(48), 48, 70);
        let retrieved = get_delegation_proof_batch_with_root_signature(
            batch_get_request(&response),
            p(1),
            60,
            |_cert_hash| Ok(root_proof(13)),
        )
        .expect("prepared batch should retrieve proofs before expiry");

        assert_eq!(
            preflight_delegation_proof_batch_install_proof(
                response.batch_id,
                &retrieved.proofs[0],
                70,
            ),
            Err(RootDelegationProofInstallOutcome::ExpiredOrSuperseded)
        );
    }

    #[test]
    fn batch_install_preflight_reports_already_installed_after_success_mark() {
        let response = prepared_batch(p(49), 49, 70);
        let retrieved = get_delegation_proof_batch_with_root_signature(
            batch_get_request(&response),
            p(1),
            60,
            |_cert_hash| Ok(root_proof(14)),
        )
        .expect("prepared batch should retrieve proofs before expiry");
        let proof = &retrieved.proofs[0];
        assert_eq!(
            preflight_delegation_proof_batch_install_proof(response.batch_id, proof, 60),
            Ok(())
        );

        mark_delegation_proof_batch_installed(response.batch_id, proof.issuer_pid, proof.cert_hash);

        assert_eq!(
            preflight_delegation_proof_batch_install_proof(response.batch_id, proof, 80),
            Err(RootDelegationProofInstallOutcome::AlreadyInstalled)
        );
    }
}
