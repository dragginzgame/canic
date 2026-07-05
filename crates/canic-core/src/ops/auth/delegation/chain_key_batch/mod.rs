//! Module: ops::auth::delegation::chain_key_batch
//!
//! Responsibility: build and persist root chain-key delegation batches.
//! Does not own: management-canister signing, timers, issuer install calls, or endpoint guards.
//! Boundary: deterministic preparation state for the 0.76 bridge-free renewal workflow.

mod batch_id;
mod install;
mod merkle;
mod selection;
mod signing;

use super::{
    errors::{map_prepare_delegation_cert_error, map_root_provisioning_policy_error},
    root_issuer_policy::{delegated_role_grant_views, delegation_audience_view},
    root_issuer_renewal::renewal_template_fingerprint,
};
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::policy::pure::auth::{
        RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
        RootIssuerRenewalOutcome, RootIssuerRenewalState,
        validate_root_delegation_proof_prepare_policy,
    },
    dto::auth::{
        ChainKeyBatchHeaderV1, ChainKeyDelegationCertV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchProof, RootDelegationProofInstallOutcome,
    },
    ops::{
        auth::{
            delegated::{
                canonical::{
                    chain_key_batch_header_hash, chain_key_delegation_cert_hash,
                    chain_key_derivation_path_hash,
                },
                cert_rules::DelegatedAuthTtlLimits,
                chain_key_signing::{ChainKeySigner, ChainKeySigningPolicy},
                delegation_cert::{PrepareDelegationCertInput, prepare_delegation_cert},
            },
            issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
        },
        storage::auth::{
            AuthStateOps, ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchIssuer,
            ChainKeyRootDelegationBatchStatus,
        },
    },
};
use batch_id::{ChainKeyBatchIdInput, chain_key_batch_id};
use install::{
    materialize_chain_key_delegation_proof, signed_chain_key_delegation_proof_for_issuer,
};
use merkle::{ChainKeyBatchLeaf, merkle_root_and_witnesses, reject_duplicate_chain_key_issuers};
use selection::{
    DueChainKeyTemplate, cap_due_chain_key_templates,
    chain_key_root_delegation_batch_quota_exceeded, due_chain_key_templates,
    enabled_template_count, pending_chain_key_root_delegation_batch_count,
};
pub(in crate::ops::auth) use signing::{
    sign_chain_key_root_delegation_batch, sign_next_chain_key_root_delegation_batch,
};

const CHAIN_KEY_BATCH_SCHEMA_VERSION_V1: u16 = 1;
const MAX_CHAIN_KEY_ROOT_DELEGATION_BATCH_ISSUERS: usize = 64;
const MAX_PENDING_CHAIN_KEY_ROOT_DELEGATION_BATCHES: usize = 128;
const CHAIN_KEY_SIGNING_RETRY_BACKOFF_NS: u64 = 60_000_000_000;

///
/// PrepareDueChainKeyRootDelegationBatchInput
///
/// Root-local inputs required to prepare one chain-key delegation batch.
///

pub(in crate::ops::auth) struct PrepareDueChainKeyRootDelegationBatchInput<'a> {
    pub signing_policy: &'a ChainKeySigningPolicy,
    pub max_cert_ttl_ns: u64,
    pub max_revocation_latency_ns: u64,
    pub min_accepted_proof_epoch: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
    pub required_issuer_pid: Option<Principal>,
    pub now_ns: u64,
}

///
/// PrepareDueChainKeyRootDelegationBatchResult
///
/// Summary of one chain-key batch preparation sweep.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ops::auth) struct PrepareDueChainKeyRootDelegationBatchResult {
    pub batch_id: Option<[u8; 32]>,
    pub prepared_issuers: usize,
    pub skipped_templates: usize,
    pub reused_in_flight: bool,
}

///
/// SignNextChainKeyRootDelegationBatchResult
///
/// Summary of one persisted chain-key signing step.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ops::auth) struct SignNextChainKeyRootDelegationBatchResult {
    pub batch_id: Option<[u8; 32]>,
    pub signed: bool,
    pub reused_signed: bool,
    pub signing_in_flight: bool,
}

///
/// ChainKeyRootDelegationBatchInstallPlan
///
/// Issuer-specific proof payloads materialized from one signed chain-key batch.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::ops::auth) struct ChainKeyRootDelegationBatchInstallPlan {
    pub batch_id: [u8; 32],
    pub proofs: Vec<RootDelegationProofBatchProof>,
}

pub(in crate::ops::auth) fn prepare_due_chain_key_root_delegation_batch(
    input: PrepareDueChainKeyRootDelegationBatchInput<'_>,
) -> Result<PrepareDueChainKeyRootDelegationBatchResult, InternalError> {
    AuthStateOps::prune_chain_key_root_delegation_batches(input.now_ns);
    mark_stale_preinstall_chain_key_batches(input.registry_epoch, input.registry_hash);

    if let Some(batch) = reusable_in_flight_chain_key_batch(
        input.now_ns,
        input.required_issuer_pid,
        input.registry_epoch,
        input.registry_hash,
    ) {
        return Ok(PrepareDueChainKeyRootDelegationBatchResult {
            batch_id: Some(batch.batch_id),
            prepared_issuers: batch.issuers.len(),
            skipped_templates: enabled_template_count().saturating_sub(batch.issuers.len()),
            reused_in_flight: true,
        });
    }

    let mut due_templates = due_chain_key_templates(input.now_ns, input.required_issuer_pid);
    due_templates.sort_by(|left, right| {
        left.template
            .issuer_pid
            .as_slice()
            .cmp(right.template.issuer_pid.as_slice())
    });
    cap_due_chain_key_templates(&mut due_templates);

    if due_templates.is_empty() {
        return Ok(PrepareDueChainKeyRootDelegationBatchResult {
            batch_id: None,
            prepared_issuers: 0,
            skipped_templates: enabled_template_count(),
            reused_in_flight: false,
        });
    }

    let pending_batches = pending_chain_key_root_delegation_batch_count(input.now_ns);
    if pending_batches >= MAX_PENDING_CHAIN_KEY_ROOT_DELEGATION_BATCHES {
        return Err(chain_key_root_delegation_batch_quota_exceeded(
            pending_batches,
        ));
    }

    let proof_epoch =
        AuthStateOps::advance_delegated_auth_proof_epoch_at_least(input.min_accepted_proof_epoch);
    let batch = build_chain_key_root_delegation_batch(input, &due_templates, proof_epoch)?;
    let result = PrepareDueChainKeyRootDelegationBatchResult {
        batch_id: Some(batch.batch_id),
        prepared_issuers: batch.issuers.len(),
        skipped_templates: enabled_template_count().saturating_sub(batch.issuers.len()),
        reused_in_flight: false,
    };
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    Ok(result)
}

pub(in crate::ops::auth) async fn get_or_create_chain_key_delegation_proof_for_issuer<S>(
    mut input: PrepareDueChainKeyRootDelegationBatchInput<'_>,
    issuer_pid: Principal,
    signer: &mut S,
) -> Result<Option<RootDelegationProofBatchProof>, InternalError>
where
    S: ChainKeySigner,
{
    AuthStateOps::prune_chain_key_root_delegation_batches(input.now_ns);
    if let Some(proof) = signed_chain_key_delegation_proof_for_issuer(
        issuer_pid,
        input.now_ns,
        input.registry_epoch,
        input.registry_hash,
    ) {
        return Ok(Some(proof));
    }

    let signing_policy = input.signing_policy;
    let now_ns = input.now_ns;
    let registry_epoch = input.registry_epoch;
    let registry_hash = input.registry_hash;
    input.required_issuer_pid = Some(issuer_pid);
    let prepared = prepare_due_chain_key_root_delegation_batch(input)?;
    let Some(batch_id) = prepared.batch_id else {
        return Ok(None);
    };

    let signing_result =
        sign_chain_key_root_delegation_batch(signing_policy, batch_id, now_ns, signer).await?;
    if signing_result.signing_in_flight {
        return Ok(None);
    }

    Ok(signed_chain_key_delegation_proof_for_issuer(
        issuer_pid,
        now_ns,
        registry_epoch,
        registry_hash,
    ))
}

pub(in crate::ops::auth) fn start_next_chain_key_root_delegation_batch_install(
    now_ns: u64,
) -> Result<Option<ChainKeyRootDelegationBatchInstallPlan>, InternalError> {
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(batch) = next_chain_key_batch_for_install(now_ns) else {
        return Ok(None);
    };
    start_chain_key_root_delegation_batch_install(batch.batch_id, now_ns)
}

pub(in crate::ops::auth) fn start_chain_key_root_delegation_batch_install(
    batch_id: [u8; 32],
    now_ns: u64,
) -> Result<Option<ChainKeyRootDelegationBatchInstallPlan>, InternalError> {
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(mut batch) = AuthStateOps::chain_key_root_delegation_batch(batch_id) else {
        return Ok(None);
    };
    if now_ns >= batch.header.expires_at_ns
        || !matches!(
            batch.status,
            ChainKeyRootDelegationBatchStatus::Signed
                | ChainKeyRootDelegationBatchStatus::Installing
        )
    {
        return Ok(None);
    }

    let signature = batch.signature.clone().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            "signed chain-key root delegation batch is missing a signature",
        )
    })?;
    let proofs = batch
        .issuers
        .iter()
        .filter(|issuer| issuer.installed_at_ns.is_none())
        .map(|issuer| materialize_chain_key_delegation_proof(&batch, issuer, &signature))
        .collect::<Vec<_>>();

    if proofs.is_empty() {
        batch.status = ChainKeyRootDelegationBatchStatus::Installed;
        batch.installed_at_ns.get_or_insert(now_ns);
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
        return Ok(None);
    }

    if batch.status == ChainKeyRootDelegationBatchStatus::Signed {
        batch.status = ChainKeyRootDelegationBatchStatus::Installing;
        batch.install_started_at_ns = Some(now_ns);
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    }

    Ok(Some(ChainKeyRootDelegationBatchInstallPlan {
        batch_id,
        proofs,
    }))
}

pub(in crate::ops::auth) fn record_chain_key_root_delegation_install_success(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
    now_ns: u64,
) -> bool {
    let Some(mut batch) = AuthStateOps::chain_key_root_delegation_batch(batch_id) else {
        return false;
    };
    if !matches!(
        batch.status,
        ChainKeyRootDelegationBatchStatus::Signed
            | ChainKeyRootDelegationBatchStatus::Installing
            | ChainKeyRootDelegationBatchStatus::Installed
    ) {
        return false;
    }

    let Some(index) = batch
        .issuers
        .iter()
        .position(|issuer| issuer.issuer_pid == issuer_pid && issuer.cert_hash == cert_hash)
    else {
        return false;
    };
    if batch.issuers[index].installed_at_ns.is_some() {
        return true;
    }

    batch.issuers[index].installed_at_ns = Some(now_ns);
    batch.issuers[index].last_failure = None;
    let installed_issuer = batch.issuers[index].clone();
    upsert_chain_key_issuer_installed_state(&installed_issuer, now_ns);

    if batch
        .issuers
        .iter()
        .all(|issuer| issuer.last_failure.is_none())
    {
        batch.failure = None;
    }
    if batch
        .issuers
        .iter()
        .all(|issuer| issuer.installed_at_ns.is_some())
    {
        batch.status = ChainKeyRootDelegationBatchStatus::Installed;
        batch.installed_at_ns = Some(now_ns);
        batch.failure = None;
    }
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    true
}

pub(in crate::ops::auth) fn record_chain_key_root_delegation_install_failure(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
    outcome: RootDelegationProofInstallOutcome,
) -> bool {
    let Some(mut batch) = AuthStateOps::chain_key_root_delegation_batch(batch_id) else {
        return false;
    };
    if !matches!(
        batch.status,
        ChainKeyRootDelegationBatchStatus::Signed | ChainKeyRootDelegationBatchStatus::Installing
    ) {
        return false;
    }
    let Some(index) = batch
        .issuers
        .iter()
        .position(|issuer| issuer.issuer_pid == issuer_pid && issuer.cert_hash == cert_hash)
    else {
        return false;
    };
    if batch.issuers[index].installed_at_ns.is_some() {
        return false;
    }

    let reason = format!("{outcome:?}");
    batch.issuers[index].last_failure = Some(reason.clone());
    batch.failure = Some(reason);
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    true
}

fn next_chain_key_batch_for_install(now_ns: u64) -> Option<ChainKeyRootDelegationBatch> {
    let mut batches = AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| {
            matches!(
                batch.status,
                ChainKeyRootDelegationBatchStatus::Signed
                    | ChainKeyRootDelegationBatchStatus::Installing
            )
        })
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| {
        left.prepared_at_ns
            .cmp(&right.prepared_at_ns)
            .then_with(|| left.batch_id.cmp(&right.batch_id))
    });
    batches.into_iter().next()
}

fn upsert_chain_key_issuer_installed_state(
    issuer: &ChainKeyRootDelegationBatchIssuer,
    now_ns: u64,
) {
    let template_fingerprint = AuthStateOps::root_issuer_renewal_template(issuer.issuer_pid)
        .map_or_else(
            || {
                AuthStateOps::root_issuer_renewal_state(issuer.issuer_pid)
                    .map_or([0; 32], |state| state.template_fingerprint)
            },
            |template| renewal_template_fingerprint(&template),
        );
    AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
        issuer_pid: issuer.issuer_pid,
        template_fingerprint,
        last_installed_cert_hash: Some(issuer.cert_hash),
        last_installed_expires_at_ns: Some(issuer.delegation_cert.expires_at_ns),
        last_installed_refresh_after_ns: Some(issuer.refresh_after_ns),
        active_attempt_id: None,
        last_outcome: RootIssuerRenewalOutcome::Installed,
        consecutive_failures: 0,
        next_attempt_after_ns: issuer.refresh_after_ns,
        updated_at_ns: now_ns,
    });
}

fn reusable_in_flight_chain_key_batch(
    now_ns: u64,
    required_issuer_pid: Option<Principal>,
    registry_epoch: u64,
    registry_hash: [u8; 32],
) -> Option<ChainKeyRootDelegationBatch> {
    let mut batches = AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| batch_matches_registry(batch, registry_epoch, registry_hash))
        .filter(|batch| {
            required_issuer_pid.is_none_or(|issuer_pid| {
                batch
                    .issuers
                    .iter()
                    .any(|issuer| issuer.issuer_pid == issuer_pid)
            })
        })
        .filter(|batch| {
            matches!(
                batch.status,
                ChainKeyRootDelegationBatchStatus::Prepared
                    | ChainKeyRootDelegationBatchStatus::Signing
                    | ChainKeyRootDelegationBatchStatus::Signed
                    | ChainKeyRootDelegationBatchStatus::Installing
                    | ChainKeyRootDelegationBatchStatus::FailedRetryable
            )
        })
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| {
        left.prepared_at_ns
            .cmp(&right.prepared_at_ns)
            .then_with(|| left.batch_id.cmp(&right.batch_id))
    });
    batches.into_iter().next()
}

fn mark_stale_preinstall_chain_key_batches(registry_epoch: u64, registry_hash: [u8; 32]) -> usize {
    let mut stale_count = 0usize;
    for mut batch in AuthStateOps::chain_key_root_delegation_batches() {
        if batch.status == ChainKeyRootDelegationBatchStatus::Installed
            || batch_matches_registry(&batch, registry_epoch, registry_hash)
        {
            continue;
        }
        batch.status = ChainKeyRootDelegationBatchStatus::FailedRetryable;
        batch.retry_after_ns = Some(batch.header.expires_at_ns);
        batch.failure = Some("stale registry epoch or hash".to_string());
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
        stale_count += 1;
    }
    stale_count
}

fn batch_matches_registry(
    batch: &ChainKeyRootDelegationBatch,
    registry_epoch: u64,
    registry_hash: [u8; 32],
) -> bool {
    batch.header.registry_epoch == registry_epoch && batch.header.registry_hash == registry_hash
}

fn build_chain_key_root_delegation_batch(
    input: PrepareDueChainKeyRootDelegationBatchInput<'_>,
    due_templates: &[DueChainKeyTemplate],
    proof_epoch: u64,
) -> Result<ChainKeyRootDelegationBatch, InternalError> {
    let cert_ttl_ns = shared_batch_cert_ttl_ns(
        due_templates,
        input.max_cert_ttl_ns,
        input.max_revocation_latency_ns,
    )?;
    let expires_at_ns = input.now_ns.checked_add(cert_ttl_ns).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            "chain-key root delegation batch expiry overflow",
        )
    })?;

    let mut leaves = due_templates
        .iter()
        .map(|due| build_chain_key_batch_leaf(&input, due, proof_epoch, cert_ttl_ns, expires_at_ns))
        .collect::<Result<Vec<_>, _>>()?;
    leaves.sort_by(|left, right| {
        left.delegation_cert
            .issuer_pid
            .as_slice()
            .cmp(right.delegation_cert.issuer_pid.as_slice())
    });
    reject_duplicate_chain_key_issuers(&leaves)?;

    let leaf_hashes = leaves.iter().map(|leaf| leaf.leaf_hash).collect::<Vec<_>>();
    let (tree_root, witnesses) = merkle_root_and_witnesses(&leaf_hashes)?;
    let derivation_path_hash =
        chain_key_derivation_path_hash(&input.signing_policy.derivation_path);
    let batch_id = chain_key_batch_id(ChainKeyBatchIdInput {
        root_canister_id: input.signing_policy.root_canister_id,
        proof_epoch,
        registry_epoch: input.registry_epoch,
        registry_hash: input.registry_hash,
        tree_root,
        not_before_ns: input.now_ns,
        expires_at_ns,
        algorithm: input.signing_policy.algorithm,
        key_id_name: &input.signing_policy.key_id.name,
        derivation_path_hash,
        key_version: input.signing_policy.key_version,
    });
    let header = ChainKeyBatchHeaderV1 {
        schema_version: CHAIN_KEY_BATCH_SCHEMA_VERSION_V1,
        root_canister_id: input.signing_policy.root_canister_id,
        batch_id,
        proof_epoch,
        registry_epoch: input.registry_epoch,
        registry_hash: input.registry_hash,
        tree_root,
        not_before_ns: input.now_ns,
        expires_at_ns,
        algorithm: input.signing_policy.algorithm,
        key_id: input.signing_policy.key_id.clone(),
        derivation_path_hash,
        key_version: input.signing_policy.key_version,
    };

    let issuers = leaves
        .into_iter()
        .zip(witnesses)
        .map(|(leaf, issuer_witness)| ChainKeyRootDelegationBatchIssuer {
            issuer_pid: leaf.delegation_cert.issuer_pid,
            cert_hash: leaf.cert_hash,
            delegation_cert: leaf.delegation_cert,
            chain_key_delegation_cert: leaf.chain_key_delegation_cert,
            issuer_witness,
            refresh_after_ns: leaf.refresh_after_ns,
            installed_at_ns: None,
            last_failure: None,
        })
        .collect();

    Ok(ChainKeyRootDelegationBatch {
        batch_id,
        status: ChainKeyRootDelegationBatchStatus::Prepared,
        header_hash: chain_key_batch_header_hash(&header),
        header,
        signature: None,
        issuers,
        prepared_at_ns: input.now_ns,
        signed_at_ns: None,
        install_started_at_ns: None,
        installed_at_ns: None,
        retry_after_ns: None,
        failure: None,
    })
}

fn shared_batch_cert_ttl_ns(
    due_templates: &[DueChainKeyTemplate],
    max_cert_ttl_ns: u64,
    max_revocation_latency_ns: u64,
) -> Result<u64, InternalError> {
    let template_ttl_ns = due_templates
        .iter()
        .map(|due| due.template.cert_ttl_ns)
        .min()
        .ok_or_else(|| InternalError::invalid_input("chain-key batch must include an issuer"))?;
    let ttl_ns = template_ttl_ns
        .min(max_cert_ttl_ns)
        .min(max_revocation_latency_ns);
    if ttl_ns == 0 {
        return Err(InternalError::invalid_input(
            "chain-key root delegation batch TTL must be greater than zero",
        ));
    }
    Ok(ttl_ns)
}

fn build_chain_key_batch_leaf(
    input: &PrepareDueChainKeyRootDelegationBatchInput<'_>,
    due: &DueChainKeyTemplate,
    proof_epoch: u64,
    cert_ttl_ns: u64,
    expires_at_ns: u64,
) -> Result<ChainKeyBatchLeaf, InternalError> {
    let policy = AuthStateOps::root_issuer_policy(due.template.issuer_pid);
    let audience = delegation_audience_view(&due.template.audience);
    let grants = delegated_role_grant_views(&due.template.grants);
    let decision = validate_root_delegation_proof_prepare_policy(
        policy.as_ref(),
        RootDelegationProofPreparePolicyInput {
            issuer_pid: due.template.issuer_pid,
            audience: &due.template.audience,
            grants: &due.template.grants,
            cert_ttl_ns,
            issued_at_ns: input.now_ns,
        },
    )
    .map_err(map_root_provisioning_policy_error)?;
    ensure_policy_decision_matches_shared_window(decision, expires_at_ns)?;

    let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
        seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
    };
    let prepared = prepare_delegation_cert(PrepareDelegationCertInput {
        root_pid: input.signing_policy.root_canister_id,
        issuer_pid: due.template.issuer_pid,
        issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
        issuer_proof_binding,
        issued_at_ns: input.now_ns,
        cert_ttl_ns,
        max_token_ttl_ns: cert_ttl_ns,
        audience,
        grants,
        ttl_limits: DelegatedAuthTtlLimits {
            max_cert_ttl_ns: input.max_cert_ttl_ns.min(input.max_revocation_latency_ns),
            max_token_ttl_ns: cert_ttl_ns,
        },
    })
    .map_err(map_prepare_delegation_cert_error)?;

    let chain_key_delegation_cert = ChainKeyDelegationCertV1 {
        root_canister_id: prepared.cert.root_pid,
        issuer_canister_id: prepared.cert.issuer_pid,
        proof_epoch,
        issuer_proof_algorithm: prepared.cert.issuer_proof_alg,
        issuer_proof_binding_hash: prepared.cert.issuer_proof_binding_hash,
        issuer_proof_binding: prepared.cert.issuer_proof_binding,
        max_token_ttl_ns: prepared.cert.max_token_ttl_ns,
        audience: prepared.cert.aud.clone(),
        grants: prepared.cert.grants.clone(),
        not_before_ns: prepared.cert.not_before_ns,
        expires_at_ns: prepared.cert.expires_at_ns,
        registry_epoch: input.registry_epoch,
        registry_hash: input.registry_hash,
    };
    let leaf_hash = chain_key_delegation_cert_hash(&chain_key_delegation_cert).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("chain-key delegation cert canonicalization failed: {err}"),
        )
    })?;

    Ok(ChainKeyBatchLeaf {
        delegation_cert: prepared.cert,
        chain_key_delegation_cert,
        cert_hash: prepared.cert_hash,
        leaf_hash,
        refresh_after_ns: decision.refresh_after_ns,
    })
}

fn ensure_policy_decision_matches_shared_window(
    decision: RootDelegationProofPreparePolicyDecision,
    expires_at_ns: u64,
) -> Result<(), InternalError> {
    if decision.expires_at_ns != expires_at_ns {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Ops,
            "chain-key root delegation policy decision expiry mismatch",
        ));
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests;
