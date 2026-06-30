//! Module: ops::auth::delegation::chain_key_batch
//!
//! Responsibility: build and persist root chain-key delegation batches.
//! Does not own: management-canister signing, timers, issuer install calls, or endpoint guards.
//! Boundary: deterministic preparation state for the 0.76 bridge-free renewal workflow.

use super::{
    errors::{map_prepare_delegation_cert_error, map_root_provisioning_policy_error},
    root_issuer_policy::{delegated_role_grant_views, delegation_audience_view},
    root_issuer_renewal::renewal_template_fingerprint,
};
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
        RootIssuerRenewalOutcome, RootIssuerRenewalState, RootIssuerRenewalTemplate,
        validate_root_delegation_proof_prepare_policy,
    },
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1,
        ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1, ChainKeyRootSignatureV1, DelegationCert,
        DelegationProof, IcChainKeyBatchSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchProof, RootDelegationProofInstallOutcome, RootProof,
    },
    ops::{
        auth::{
            delegated::{
                canonical::{
                    chain_key_batch_header_hash, chain_key_delegation_cert_hash,
                    chain_key_derivation_path_hash,
                },
                cert_rules::DelegatedAuthTtlLimits,
                chain_key_signing::{
                    ChainKeySigner, ChainKeySigningPolicy, SignChainKeyBatchHeaderInput,
                    sign_chain_key_batch_header,
                },
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
use sha2::{Digest, Sha256};

const CHAIN_KEY_BATCH_SCHEMA_VERSION_V1: u16 = 1;
const CHAIN_KEY_BATCH_ID_DOMAIN: &[u8] = b"CANIC_ROOT_DELEGATION_CHAIN_KEY_BATCH_ID_V1";
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

    if due_templates.is_empty() {
        return Ok(PrepareDueChainKeyRootDelegationBatchResult {
            batch_id: None,
            prepared_issuers: 0,
            skipped_templates: enabled_template_count(),
            reused_in_flight: false,
        });
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

pub(in crate::ops::auth) async fn sign_next_chain_key_root_delegation_batch<S>(
    signing_policy: &ChainKeySigningPolicy,
    now_ns: u64,
    signer: &mut S,
) -> Result<SignNextChainKeyRootDelegationBatchResult, InternalError>
where
    S: ChainKeySigner,
{
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(batch) = next_chain_key_batch_for_signing(now_ns) else {
        return Ok(SignNextChainKeyRootDelegationBatchResult {
            batch_id: None,
            signed: false,
            reused_signed: false,
            signing_in_flight: false,
        });
    };
    sign_chain_key_root_delegation_batch(signing_policy, batch.batch_id, now_ns, signer).await
}

#[expect(
    clippy::too_many_lines,
    reason = "the retry-safe signing transition keeps the state read, callback validation, and persistence update together"
)]
pub(in crate::ops::auth) async fn sign_chain_key_root_delegation_batch<S>(
    signing_policy: &ChainKeySigningPolicy,
    batch_id: [u8; 32],
    now_ns: u64,
    signer: &mut S,
) -> Result<SignNextChainKeyRootDelegationBatchResult, InternalError>
where
    S: ChainKeySigner,
{
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(mut batch) = AuthStateOps::chain_key_root_delegation_batch(batch_id) else {
        return Ok(SignNextChainKeyRootDelegationBatchResult {
            batch_id: None,
            signed: false,
            reused_signed: false,
            signing_in_flight: false,
        });
    };
    if now_ns >= batch.header.expires_at_ns {
        return Ok(SignNextChainKeyRootDelegationBatchResult {
            batch_id: None,
            signed: false,
            reused_signed: false,
            signing_in_flight: false,
        });
    }
    match batch.status {
        ChainKeyRootDelegationBatchStatus::Signed
        | ChainKeyRootDelegationBatchStatus::Installing => {
            return Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: Some(batch.batch_id),
                signed: false,
                reused_signed: true,
                signing_in_flight: false,
            });
        }
        ChainKeyRootDelegationBatchStatus::Signing => {
            return Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: Some(batch.batch_id),
                signed: false,
                reused_signed: false,
                signing_in_flight: true,
            });
        }
        ChainKeyRootDelegationBatchStatus::Prepared
        | ChainKeyRootDelegationBatchStatus::FailedRetryable => {}
        ChainKeyRootDelegationBatchStatus::Installed => {
            return Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: None,
                signed: false,
                reused_signed: false,
                signing_in_flight: false,
            });
        }
    }

    batch.status = ChainKeyRootDelegationBatchStatus::Signing;
    batch.retry_after_ns = None;
    batch.failure = None;
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch.clone());

    match sign_chain_key_batch_header(
        SignChainKeyBatchHeaderInput {
            header: &batch.header,
            policy: signing_policy,
        },
        signer,
    )
    .await
    {
        Ok(signature) => {
            let Some(current) = AuthStateOps::chain_key_root_delegation_batch(batch.batch_id)
            else {
                return Ok(SignNextChainKeyRootDelegationBatchResult {
                    batch_id: None,
                    signed: false,
                    reused_signed: false,
                    signing_in_flight: false,
                });
            };
            if current.status != ChainKeyRootDelegationBatchStatus::Signing
                || current.header_hash != batch.header_hash
            {
                return Ok(SignNextChainKeyRootDelegationBatchResult {
                    batch_id: Some(batch.batch_id),
                    signed: false,
                    reused_signed: false,
                    signing_in_flight: false,
                });
            }
            batch = current;
            batch.status = ChainKeyRootDelegationBatchStatus::Signed;
            batch.signature = Some(signature);
            batch.signed_at_ns = Some(now_ns);
            batch.retry_after_ns = None;
            batch.failure = None;
            AuthStateOps::upsert_chain_key_root_delegation_batch(batch.clone());
            Ok(SignNextChainKeyRootDelegationBatchResult {
                batch_id: Some(batch.batch_id),
                signed: true,
                reused_signed: false,
                signing_in_flight: false,
            })
        }
        Err(err) => {
            batch.status = ChainKeyRootDelegationBatchStatus::FailedRetryable;
            batch.retry_after_ns = Some(chain_key_signing_retry_after_ns(
                now_ns,
                batch.header.expires_at_ns,
            ));
            batch.failure = Some(err.to_string());
            AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
            Err(InternalError::ops(
                InternalErrorOrigin::Ops,
                format!("chain-key root delegation batch signing failed: {err}"),
            ))
        }
    }
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
    let Some(index) = batch
        .issuers
        .iter()
        .position(|issuer| issuer.issuer_pid == issuer_pid && issuer.cert_hash == cert_hash)
    else {
        return false;
    };

    let reason = format!("{outcome:?}");
    batch.issuers[index].last_failure = Some(reason.clone());
    batch.failure = Some(reason);
    AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
    true
}

fn signed_chain_key_delegation_proof_for_issuer(
    issuer_pid: Principal,
    now_ns: u64,
    registry_epoch: u64,
    registry_hash: [u8; 32],
) -> Option<RootDelegationProofBatchProof> {
    let mut batches = AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| batch_matches_registry(batch, registry_epoch, registry_hash))
        .filter(|batch| {
            matches!(
                batch.status,
                ChainKeyRootDelegationBatchStatus::Signed
                    | ChainKeyRootDelegationBatchStatus::Installing
                    | ChainKeyRootDelegationBatchStatus::Installed
            )
        })
        .filter(|batch| batch.signature.is_some())
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| {
        right
            .header
            .proof_epoch
            .cmp(&left.header.proof_epoch)
            .then_with(|| right.prepared_at_ns.cmp(&left.prepared_at_ns))
            .then_with(|| right.batch_id.cmp(&left.batch_id))
    });

    for batch in batches {
        let Some(signature) = batch.signature.clone() else {
            continue;
        };
        if let Some(issuer) = batch
            .issuers
            .iter()
            .find(|issuer| issuer.issuer_pid == issuer_pid)
        {
            return Some(materialize_chain_key_delegation_proof(
                &batch, issuer, &signature,
            ));
        }
    }
    None
}

fn materialize_chain_key_delegation_proof(
    batch: &ChainKeyRootDelegationBatch,
    issuer: &ChainKeyRootDelegationBatchIssuer,
    signature: &ChainKeyRootSignatureV1,
) -> RootDelegationProofBatchProof {
    RootDelegationProofBatchProof {
        issuer_pid: issuer.issuer_pid,
        cert_hash: issuer.cert_hash,
        proof: DelegationProof {
            cert: issuer.delegation_cert.clone(),
            root_proof: RootProof::IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1 {
                header: batch.header.clone(),
                delegation_cert: issuer.chain_key_delegation_cert.clone(),
                issuer_witness: issuer.issuer_witness.clone(),
                signature: signature.clone(),
            }),
        },
    }
}

fn next_chain_key_batch_for_signing(now_ns: u64) -> Option<ChainKeyRootDelegationBatch> {
    let mut batches = AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| match batch.status {
            ChainKeyRootDelegationBatchStatus::Prepared
            | ChainKeyRootDelegationBatchStatus::Signing
            | ChainKeyRootDelegationBatchStatus::Signed
            | ChainKeyRootDelegationBatchStatus::Installing => true,
            ChainKeyRootDelegationBatchStatus::FailedRetryable => batch
                .retry_after_ns
                .is_none_or(|retry_after_ns| now_ns >= retry_after_ns),
            ChainKeyRootDelegationBatchStatus::Installed => false,
        })
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| {
        left.prepared_at_ns
            .cmp(&right.prepared_at_ns)
            .then_with(|| left.batch_id.cmp(&right.batch_id))
    });
    batches.into_iter().next()
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

fn chain_key_signing_retry_after_ns(now_ns: u64, expires_at_ns: u64) -> u64 {
    let backed_off = now_ns.saturating_add(CHAIN_KEY_SIGNING_RETRY_BACKOFF_NS);
    let last_retryable_ns = expires_at_ns.saturating_sub(1);
    backed_off.min(last_retryable_ns).max(now_ns)
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

#[derive(Clone)]
struct DueChainKeyTemplate {
    template: RootIssuerRenewalTemplate,
}

fn due_chain_key_templates(
    now_ns: u64,
    required_issuer_pid: Option<Principal>,
) -> Vec<DueChainKeyTemplate> {
    AuthStateOps::root_issuer_renewal_templates()
        .into_iter()
        .filter(|template| template.enabled)
        .filter_map(|template| {
            if required_issuer_pid.is_some_and(|issuer_pid| issuer_pid == template.issuer_pid) {
                return Some(DueChainKeyTemplate { template });
            }
            let template_fingerprint = renewal_template_fingerprint(&template);
            let state = AuthStateOps::root_issuer_renewal_state(template.issuer_pid);
            chain_key_template_due(now_ns, template_fingerprint, state.as_ref())
                .then_some(DueChainKeyTemplate { template })
        })
        .collect()
}

fn chain_key_template_due(
    now_ns: u64,
    template_fingerprint: [u8; 32],
    state: Option<&RootIssuerRenewalState>,
) -> bool {
    let Some(state) = state else {
        return true;
    };
    if now_ns < state.next_attempt_after_ns {
        return false;
    }
    if state.template_fingerprint != template_fingerprint {
        return true;
    }
    state
        .last_installed_refresh_after_ns
        .is_none_or(|refresh_after_ns| now_ns >= refresh_after_ns)
}

fn enabled_template_count() -> usize {
    AuthStateOps::root_issuer_renewal_templates()
        .into_iter()
        .filter(|template| template.enabled)
        .count()
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

struct ChainKeyBatchLeaf {
    delegation_cert: DelegationCert,
    chain_key_delegation_cert: ChainKeyDelegationCertV1,
    cert_hash: [u8; 32],
    leaf_hash: [u8; 32],
    refresh_after_ns: u64,
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

fn reject_duplicate_chain_key_issuers(leaves: &[ChainKeyBatchLeaf]) -> Result<(), InternalError> {
    let mut previous: Option<Principal> = None;
    for leaf in leaves {
        if previous.is_some_and(|issuer| issuer == leaf.delegation_cert.issuer_pid) {
            return Err(InternalError::invalid_input(
                "chain-key root delegation batch contains duplicate issuer",
            ));
        }
        previous = Some(leaf.delegation_cert.issuer_pid);
    }
    Ok(())
}

fn merkle_root_and_witnesses(
    leaf_hashes: &[[u8; 32]],
) -> Result<([u8; 32], Vec<ChainKeyBatchWitnessV1>), InternalError> {
    if leaf_hashes.is_empty() {
        return Err(InternalError::invalid_input(
            "chain-key Merkle batch must contain at least one leaf",
        ));
    }

    let mut witnesses = vec![Vec::new(); leaf_hashes.len()];
    let mut level = leaf_hashes
        .iter()
        .copied()
        .enumerate()
        .map(|(index, hash)| MerkleNode {
            hash,
            leaf_indices: vec![index],
        })
        .collect::<Vec<_>>();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            if pair.len() == 1 {
                next.push(pair[0].clone());
                continue;
            }

            let left = &pair[0];
            let right = &pair[1];
            for index in &left.leaf_indices {
                witnesses[*index].push(ChainKeyBatchWitnessStepV1::RightSibling(right.hash));
            }
            for index in &right.leaf_indices {
                witnesses[*index].push(ChainKeyBatchWitnessStepV1::LeftSibling(left.hash));
            }
            let mut leaf_indices =
                Vec::with_capacity(left.leaf_indices.len() + right.leaf_indices.len());
            leaf_indices.extend_from_slice(&left.leaf_indices);
            leaf_indices.extend_from_slice(&right.leaf_indices);
            next.push(MerkleNode {
                hash: chain_key_batch_node_hash(left.hash, right.hash),
                leaf_indices,
            });
        }
        level = next;
    }

    Ok((
        level[0].hash,
        witnesses
            .into_iter()
            .map(|steps| ChainKeyBatchWitnessV1 { steps })
            .collect(),
    ))
}

#[derive(Clone)]
struct MerkleNode {
    hash: [u8; 32],
    leaf_indices: Vec<usize>,
}

fn chain_key_batch_node_hash(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([1]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

struct ChainKeyBatchIdInput<'a> {
    root_canister_id: Principal,
    proof_epoch: u64,
    registry_epoch: u64,
    registry_hash: [u8; 32],
    tree_root: [u8; 32],
    not_before_ns: u64,
    expires_at_ns: u64,
    algorithm: ChainKeyAlgorithm,
    key_id_name: &'a str,
    derivation_path_hash: [u8; 32],
    key_version: u64,
}

fn chain_key_batch_id(input: ChainKeyBatchIdInput<'_>) -> [u8; 32] {
    let mut payload = Vec::with_capacity(256);
    encode_principal(&mut payload, input.root_canister_id);
    encode_u64(&mut payload, input.proof_epoch);
    encode_u64(&mut payload, input.registry_epoch);
    encode_fixed_32(&mut payload, input.registry_hash);
    encode_fixed_32(&mut payload, input.tree_root);
    encode_u64(&mut payload, input.not_before_ns);
    encode_u64(&mut payload, input.expires_at_ns);
    encode_chain_key_algorithm(&mut payload, input.algorithm);
    encode_string(&mut payload, input.key_id_name);
    encode_fixed_32(&mut payload, input.derivation_path_hash);
    encode_u64(&mut payload, input.key_version);

    let mut hasher = Sha256::new();
    hasher.update(CHAIN_KEY_BATCH_ID_DOMAIN);
    encode_bytes_for_hash(&mut hasher, &payload);
    hasher.finalize().into()
}

fn encode_chain_key_algorithm(out: &mut Vec<u8>, algorithm: ChainKeyAlgorithm) {
    let tag = match algorithm {
        ChainKeyAlgorithm::EcdsaSecp256k1 => 1,
    };
    out.push(tag);
}

fn encode_principal(out: &mut Vec<u8>, principal: Principal) {
    encode_bytes(out, principal.as_slice());
}

fn encode_string(out: &mut Vec<u8>, value: &str) {
    encode_bytes(out, value.as_bytes());
}

fn encode_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    encode_len(out, bytes.len());
    out.extend_from_slice(bytes);
}

fn encode_fixed_32(out: &mut Vec<u8>, bytes: [u8; 32]) {
    out.extend_from_slice(&bytes);
}

fn encode_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn encode_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("chain-key canonical vector length exceeds u32");
    out.extend_from_slice(&len.to_be_bytes());
}

fn encode_bytes_for_hash(hasher: &mut Sha256, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("chain-key canonical vector length exceeds u32");
    hasher.update(len.to_be_bytes());
    hasher.update(bytes);
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::policy::auth::{
            RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
            RootIssuerRenewalOutcome,
        },
        dto::auth::ChainKeyKeyId,
        ids::{BuildNetwork, CanisterRole},
        ops::auth::delegated::chain_key::{
            ChainKeyRootVerifierPolicy, ChainKeySignatureVerificationInput,
            verify_chain_key_batch_root_proof, verify_chain_key_ecdsa_signature,
        },
        ops::{
            auth::delegated::chain_key_signing::ChainKeySignerFuture,
            ic::mgmt::{
                EcdsaPublicKeyArgs, EcdsaPublicKeyResult, SignWithEcdsaArgs, SignWithEcdsaResult,
            },
        },
    };
    use futures::executor::block_on;
    use k256::ecdsa::{
        Signature as K256TestSignature, SigningKey as K256SigningKey,
        signature::hazmat::PrehashSigner,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn signing_key() -> K256SigningKey {
        K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse")
    }

    fn signing_policy() -> ChainKeySigningPolicy {
        ChainKeySigningPolicy {
            root_canister_id: p(1),
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path: vec![b"canic".to_vec(), b"delegation".to_vec()],
            public_key: signing_key()
                .verifying_key()
                .to_encoded_point(true)
                .as_bytes()
                .to_vec(),
            key_version: 4,
            build_network: BuildNetwork::Local,
            allow_test_chain_key: true,
        }
    }

    fn policy(issuer_pid: Principal) -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![RootDelegationAudiencePolicy::Project("test".to_string())],
            allowed_grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["read".to_string()],
            }],
            max_cert_ttl_ns: 120_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn template(issuer_pid: Principal, cert_ttl_ns: u64) -> RootIssuerRenewalTemplate {
        RootIssuerRenewalTemplate {
            issuer_pid,
            enabled: true,
            audience: RootDelegationAudiencePolicy::Project("test".to_string()),
            grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["read".to_string()],
            }],
            cert_ttl_ns,
        }
    }

    fn input(
        signing_policy: &ChainKeySigningPolicy,
    ) -> PrepareDueChainKeyRootDelegationBatchInput<'_> {
        PrepareDueChainKeyRootDelegationBatchInput {
            signing_policy,
            max_cert_ttl_ns: 120_000_000_000,
            max_revocation_latency_ns: 60_000_000_000,
            min_accepted_proof_epoch: 10,
            registry_epoch: 11,
            registry_hash: [22; 32],
            required_issuer_pid: None,
            now_ns: 1_000,
        }
    }

    fn verifier_policy(signing_policy: &ChainKeySigningPolicy) -> ChainKeyRootVerifierPolicy {
        ChainKeyRootVerifierPolicy {
            root_canister_id: signing_policy.root_canister_id,
            algorithm: signing_policy.algorithm,
            key_id: signing_policy.key_id.clone(),
            derivation_path_hash: chain_key_derivation_path_hash(&signing_policy.derivation_path),
            public_key: signing_policy.public_key.clone(),
            key_version: signing_policy.key_version,
            min_accepted_key_version: signing_policy.key_version,
            min_accepted_proof_epoch: 10,
            min_accepted_registry_epoch: 11,
            valid_from_ns: 1,
            accept_until_ns: 120_000_000_000,
            build_network: BuildNetwork::Local,
            allow_test_chain_key: true,
            max_revocation_latency_ns: 60_000_000_000,
        }
    }

    fn sign_header(header: &ChainKeyBatchHeaderV1) -> crate::dto::auth::ChainKeyRootSignatureV1 {
        let signature: K256TestSignature = signing_key()
            .sign_prehash(&chain_key_batch_header_hash(header))
            .expect("test prehash signature should sign");
        let policy = signing_policy();
        crate::dto::auth::ChainKeyRootSignatureV1 {
            algorithm: policy.algorithm,
            key_id: policy.key_id,
            derivation_path: policy.derivation_path,
            public_key: policy.public_key,
            signature: signature.to_bytes().to_vec(),
        }
    }

    struct MockSigner {
        public_key: Vec<u8>,
        signature: Vec<u8>,
        public_key_calls: usize,
        sign_calls: usize,
    }

    impl MockSigner {
        fn valid_for(header: &ChainKeyBatchHeaderV1) -> Self {
            let signature = sign_header(header);
            Self {
                public_key: signature.public_key,
                signature: signature.signature,
                public_key_calls: 0,
                sign_calls: 0,
            }
        }
    }

    impl ChainKeySigner for MockSigner {
        fn ecdsa_public_key(
            &mut self,
            _args: EcdsaPublicKeyArgs,
        ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
            self.public_key_calls += 1;
            Box::pin(async move {
                Ok(EcdsaPublicKeyResult {
                    public_key: self.public_key.clone(),
                    chain_code: vec![9; 32],
                })
            })
        }

        fn sign_with_ecdsa(
            &mut self,
            _args: SignWithEcdsaArgs,
        ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
            self.sign_calls += 1;
            Box::pin(async move {
                Ok(SignWithEcdsaResult {
                    signature: self.signature.clone(),
                })
            })
        }
    }

    struct DynamicMockSigner {
        public_key_calls: usize,
        sign_calls: usize,
    }

    impl ChainKeySigner for DynamicMockSigner {
        fn ecdsa_public_key(
            &mut self,
            _args: EcdsaPublicKeyArgs,
        ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
            self.public_key_calls += 1;
            Box::pin(async move {
                Ok(EcdsaPublicKeyResult {
                    public_key: signing_policy().public_key,
                    chain_code: vec![9; 32],
                })
            })
        }

        fn sign_with_ecdsa(
            &mut self,
            args: SignWithEcdsaArgs,
        ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
            self.sign_calls += 1;
            Box::pin(async move {
                let signature: K256TestSignature = signing_key()
                    .sign_prehash(&args.message_hash)
                    .expect("test prehash signature should sign");
                Ok(SignWithEcdsaResult {
                    signature: signature.to_bytes().to_vec(),
                })
            })
        }
    }

    struct StaleDuringSignSigner {
        batch_id: [u8; 32],
        public_key_calls: usize,
        sign_calls: usize,
    }

    impl ChainKeySigner for StaleDuringSignSigner {
        fn ecdsa_public_key(
            &mut self,
            _args: EcdsaPublicKeyArgs,
        ) -> ChainKeySignerFuture<'_, EcdsaPublicKeyResult> {
            self.public_key_calls += 1;
            Box::pin(async move {
                Ok(EcdsaPublicKeyResult {
                    public_key: signing_policy().public_key,
                    chain_code: vec![9; 32],
                })
            })
        }

        fn sign_with_ecdsa(
            &mut self,
            args: SignWithEcdsaArgs,
        ) -> ChainKeySignerFuture<'_, SignWithEcdsaResult> {
            self.sign_calls += 1;
            Box::pin(async move {
                let mut batch = AuthStateOps::chain_key_root_delegation_batch(self.batch_id)
                    .expect("batch should still exist while signing");
                batch.status = ChainKeyRootDelegationBatchStatus::FailedRetryable;
                batch.retry_after_ns = Some(batch.header.expires_at_ns);
                batch.failure = Some("stale registry epoch or hash".to_string());
                AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

                let signature: K256TestSignature = signing_key()
                    .sign_prehash(&args.message_hash)
                    .expect("test prehash signature should sign");
                Ok(SignWithEcdsaResult {
                    signature: signature.to_bytes().to_vec(),
                })
            })
        }
    }

    #[test]
    fn chain_key_batch_builder_prepares_merkle_batch_that_verifier_accepts() {
        let signing_policy = signing_policy();
        let issuer_a = p(42);
        let issuer_b = p(41);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_a));
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_b));

        let batch = build_chain_key_root_delegation_batch(
            input(&signing_policy),
            &[
                DueChainKeyTemplate {
                    template: template(issuer_a, 90_000_000_000),
                },
                DueChainKeyTemplate {
                    template: template(issuer_b, 60_000_000_000),
                },
            ],
            10,
        )
        .expect("batch should build");

        assert_eq!(batch.status, ChainKeyRootDelegationBatchStatus::Prepared);
        assert_eq!(batch.issuers.len(), 2);
        assert_eq!(batch.header.not_before_ns, 1_000);
        assert_eq!(batch.header.expires_at_ns, 60_000_001_000);
        assert_eq!(
            batch.header_hash,
            chain_key_batch_header_hash(&batch.header)
        );
        assert_eq!(batch.issuers[0].issuer_pid, issuer_b);
        assert_eq!(batch.issuers[1].issuer_pid, issuer_a);

        let signature = sign_header(&batch.header);
        for issuer in &batch.issuers {
            let proof = crate::dto::auth::RootProof::IcChainKeyBatchSignatureV1(
                crate::dto::auth::IcChainKeyBatchSignatureProofV1 {
                    header: batch.header.clone(),
                    delegation_cert: issuer.chain_key_delegation_cert.clone(),
                    issuer_witness: issuer.issuer_witness.clone(),
                    signature: signature.clone(),
                },
            );

            verify_chain_key_batch_root_proof(
                crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
                    cert: &issuer.delegation_cert,
                    root_proof: &proof,
                    policy: &verifier_policy(&signing_policy),
                    now_ns: 1_000,
                },
                |input: ChainKeySignatureVerificationInput<'_>| {
                    verify_chain_key_ecdsa_signature(input)
                },
            )
            .expect("builder proof material should verify");
        }
    }

    #[test]
    fn chain_key_batch_builder_rejects_duplicate_issuer_leaves() {
        let signing_policy = signing_policy();
        let issuer = p(43);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));

        let err = build_chain_key_root_delegation_batch(
            input(&signing_policy),
            &[
                DueChainKeyTemplate {
                    template: template(issuer, 90_000_000_000),
                },
                DueChainKeyTemplate {
                    template: template(issuer, 60_000_000_000),
                },
            ],
            10,
        )
        .expect_err("duplicate issuer leaves must reject");

        assert!(err.to_string().contains("duplicate issuer"));
    }

    #[test]
    fn chain_key_batch_prepare_reuses_in_flight_batch() {
        let signing_policy = signing_policy();
        let issuer = p(50);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));

        let first = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("first prepare should build");
        let second = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("second prepare should reuse");

        assert!(first.batch_id.is_some());
        assert_eq!(first.prepared_issuers, 1);
        assert!(!first.reused_in_flight);
        assert_eq!(second.batch_id, first.batch_id);
        assert_eq!(second.prepared_issuers, 1);
        assert!(second.reused_in_flight);
        assert_eq!(AuthStateOps::chain_key_root_delegation_batches().len(), 1);
        assert!(AuthStateOps::chain_key_root_delegation_batch(first.batch_id.unwrap()).is_some());
    }

    #[test]
    fn chain_key_batch_signing_signs_prepared_batch_once_and_reuses_signed_state() {
        let signing_policy = signing_policy();
        let issuer = p(51);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build a batch");
        let batch = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
            .expect("prepared batch should be stored");
        let mut signer = MockSigner::valid_for(&batch.header);

        let signing_result = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            2_000,
            &mut signer,
        ))
        .expect("signing should succeed");

        assert_eq!(signing_result.batch_id, prepared.batch_id);
        assert!(signing_result.signed);
        assert!(!signing_result.reused_signed);
        assert!(!signing_result.signing_in_flight);
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);
        let stored = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
            .expect("signed batch should remain stored");
        assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signed);
        assert!(stored.signature.is_some());
        assert_eq!(stored.signed_at_ns, Some(2_000));

        let mut second_signer = MockSigner::valid_for(&stored.header);
        let reused = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            3_000,
            &mut second_signer,
        ))
        .expect("signed batch should be reused");

        assert_eq!(reused.batch_id, prepared.batch_id);
        assert!(!reused.signed);
        assert!(reused.reused_signed);
        assert_eq!(second_signer.public_key_calls, 0);
        assert_eq!(second_signer.sign_calls, 0);
    }

    #[test]
    fn chain_key_batch_signing_covers_multiple_issuers_with_one_signature() {
        let signing_policy = signing_policy();
        let issuer_a = p(56);
        let issuer_b = p(57);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_a));
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_b));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_a, 60_000_000_000));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_b, 60_000_000_000));

        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build one multi-issuer batch");
        let batch_id = prepared.batch_id.expect("prepare should return a batch id");
        assert_eq!(prepared.prepared_issuers, 2);
        let batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("prepared multi-issuer batch should be stored");
        assert_eq!(batch.issuers.len(), 2);

        let mut signer = MockSigner::valid_for(&batch.header);
        let signing_result = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            2_000,
            &mut signer,
        ))
        .expect("multi-issuer batch signing should succeed");

        assert_eq!(signing_result.batch_id, Some(batch_id));
        assert!(signing_result.signed);
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);

        let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
            .expect("install plan should build")
            .expect("signed multi-issuer batch should produce an install plan");

        assert_eq!(plan.batch_id, batch_id);
        assert_eq!(plan.proofs.len(), 2);
        let proof_issuers = plan
            .proofs
            .iter()
            .map(|proof| proof.issuer_pid)
            .collect::<Vec<_>>();
        assert!(proof_issuers.contains(&issuer_a));
        assert!(proof_issuers.contains(&issuer_b));
        for proof in &plan.proofs {
            verify_chain_key_batch_root_proof(
                crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
                    cert: &proof.proof.cert,
                    root_proof: &proof.proof.root_proof,
                    policy: &verifier_policy(&signing_policy),
                    now_ns: 3_000,
                },
                |input: ChainKeySignatureVerificationInput<'_>| {
                    verify_chain_key_ecdsa_signature(input)
                },
            )
            .expect("each issuer proof from the shared batch should verify");
        }

        let mut reused_signer = MockSigner::valid_for(&batch.header);
        let reused = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            4_000,
            &mut reused_signer,
        ))
        .expect("signed multi-issuer batch should be reused");

        assert_eq!(reused.batch_id, Some(batch_id));
        assert!(reused.reused_signed);
        assert_eq!(reused_signer.public_key_calls, 0);
        assert_eq!(reused_signer.sign_calls, 0);
    }

    #[test]
    fn chain_key_batch_signing_failure_marks_same_batch_retryable() {
        let signing_policy = signing_policy();
        let issuer = p(52);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build a batch");
        let batch = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
            .expect("prepared batch should be stored");
        let mut signer = MockSigner::valid_for(&batch.header);
        signer.public_key[0] ^= 1;

        let err = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            2_000,
            &mut signer,
        ))
        .expect_err("public-key mismatch should fail signing");

        assert!(err.to_string().contains("signing failed"));
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 0);
        let stored = AuthStateOps::chain_key_root_delegation_batch(prepared.batch_id.unwrap())
            .expect("failed batch should remain stored");
        assert_eq!(
            stored.status,
            ChainKeyRootDelegationBatchStatus::FailedRetryable
        );
        let retry_after_ns = stored.retry_after_ns.expect("failed batch retry time");
        assert!(retry_after_ns >= 2_000);
        assert!(retry_after_ns < stored.header.expires_at_ns);
        assert!(stored.signature.is_none());

        let mut blocked_signer = MockSigner::valid_for(&stored.header);
        let blocked = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            3_000,
            &mut blocked_signer,
        ))
        .expect("retry delay should skip signing");

        assert_eq!(blocked.batch_id, None);
        assert_eq!(blocked_signer.public_key_calls, 0);
        assert_eq!(blocked_signer.sign_calls, 0);

        let mut retry_signer = MockSigner::valid_for(&stored.header);
        let retried = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            retry_after_ns,
            &mut retry_signer,
        ))
        .expect("retry delay expiry should allow the same batch to sign");

        assert_eq!(retried.batch_id, prepared.batch_id);
        assert!(retried.signed);
        assert_eq!(retry_signer.public_key_calls, 1);
        assert_eq!(retry_signer.sign_calls, 1);
        let retried_stored = AuthStateOps::chain_key_root_delegation_batch(
            prepared.batch_id.expect("prepared batch id"),
        )
        .expect("retried batch should remain stored");
        assert_eq!(
            retried_stored.status,
            ChainKeyRootDelegationBatchStatus::Signed
        );
        assert_eq!(retried_stored.retry_after_ns, None);
        assert_eq!(retried_stored.failure, None);
        assert!(retried_stored.signature.is_some());
    }

    #[test]
    fn chain_key_batch_duplicate_signing_tick_observes_in_flight_without_management_calls() {
        let signing_policy = signing_policy();
        let issuer = p(58);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build a batch");
        let batch_id = prepared.batch_id.expect("prepare should return a batch id");
        let mut batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("prepared batch should be stored");
        batch.status = ChainKeyRootDelegationBatchStatus::Signing;
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);
        let mut signer = DynamicMockSigner {
            public_key_calls: 0,
            sign_calls: 0,
        };

        let result = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            2_000,
            &mut signer,
        ))
        .expect("duplicate signing tick should be a no-op");

        assert_eq!(result.batch_id, Some(batch_id));
        assert!(result.signing_in_flight);
        assert!(!result.signed);
        assert_eq!(signer.public_key_calls, 0);
        assert_eq!(signer.sign_calls, 0);
        let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("in-flight batch should remain stored");
        assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signing);
        assert!(stored.signature.is_none());
    }

    #[test]
    fn chain_key_batch_discards_signature_returning_after_batch_became_stale() {
        let signing_policy = signing_policy();
        let issuer = p(66);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build a batch");
        let batch_id = prepared.batch_id.expect("prepare should return a batch id");
        let mut signer = StaleDuringSignSigner {
            batch_id,
            public_key_calls: 0,
            sign_calls: 0,
        };

        let result = block_on(sign_chain_key_root_delegation_batch(
            &signing_policy,
            batch_id,
            2_000,
            &mut signer,
        ))
        .expect("stale callback should be discarded without failing sweep");

        assert_eq!(result.batch_id, Some(batch_id));
        assert!(!result.signed);
        assert!(!result.reused_signed);
        assert!(!result.signing_in_flight);
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);
        let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("stale batch should remain until expiry pruning");
        assert_eq!(
            stored.status,
            ChainKeyRootDelegationBatchStatus::FailedRetryable
        );
        assert!(stored.signature.is_none());
        assert_eq!(
            stored.failure.as_deref(),
            Some("stale registry epoch or hash")
        );
    }

    #[test]
    fn chain_key_batch_registry_change_discards_stale_preinstall_batch() {
        let signing_policy = signing_policy();
        let issuer = p(59);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build the original batch");
        let stale_batch_id = prepared.batch_id.expect("original batch id");
        let mut stale_batch = AuthStateOps::chain_key_root_delegation_batch(stale_batch_id)
            .expect("original batch should be stored");
        stale_batch.status = ChainKeyRootDelegationBatchStatus::Signing;
        AuthStateOps::upsert_chain_key_root_delegation_batch(stale_batch);

        let mut changed_registry = input(&signing_policy);
        changed_registry.registry_epoch = 12;
        changed_registry.registry_hash = [33; 32];
        let refreshed = prepare_due_chain_key_root_delegation_batch(changed_registry)
            .expect("registry change should prepare a fresh batch");
        let refreshed_batch_id = refreshed.batch_id.expect("fresh batch id");

        assert_ne!(refreshed_batch_id, stale_batch_id);
        assert!(!refreshed.reused_in_flight);
        let stale_batch = AuthStateOps::chain_key_root_delegation_batch(stale_batch_id)
            .expect("stale batch should remain until expiry pruning");
        assert_eq!(
            stale_batch.status,
            ChainKeyRootDelegationBatchStatus::FailedRetryable
        );
        assert_eq!(
            stale_batch.retry_after_ns,
            Some(stale_batch.header.expires_at_ns)
        );
        assert_eq!(
            stale_batch.failure.as_deref(),
            Some("stale registry epoch or hash")
        );
        let refreshed_batch = AuthStateOps::chain_key_root_delegation_batch(refreshed_batch_id)
            .expect("fresh registry batch should be stored");
        assert_eq!(refreshed_batch.header.registry_epoch, 12);
        assert_eq!(refreshed_batch.header.registry_hash, [33; 32]);

        let mut signer = MockSigner::valid_for(&refreshed_batch.header);
        let signing_result = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            2_000,
            &mut signer,
        ))
        .expect("fresh registry batch should sign");

        assert_eq!(signing_result.batch_id, Some(refreshed_batch_id));
        assert!(signing_result.signed);
        assert_eq!(signer.sign_calls, 1);
    }

    #[test]
    fn chain_key_batch_expired_preinstall_batch_is_pruned_before_signing() {
        let signing_policy = signing_policy();
        let issuer = p(63);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build a batch");
        let batch_id = prepared.batch_id.expect("prepared batch id");
        let batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("prepared batch should be stored");
        let mut signer = MockSigner::valid_for(&batch.header);

        let result = block_on(sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            batch.header.expires_at_ns,
            &mut signer,
        ))
        .expect("expired batch should be pruned without signing");

        assert_eq!(result.batch_id, None);
        assert_eq!(signer.public_key_calls, 0);
        assert_eq!(signer.sign_calls, 0);
        assert!(
            AuthStateOps::chain_key_root_delegation_batch(batch_id).is_none(),
            "expired pre-install batch must not remain signable"
        );
    }

    #[test]
    fn chain_key_batch_install_plan_materializes_signed_proof_and_records_success() {
        let signing_policy = signing_policy();
        let issuer = p(53);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let mut batch = build_chain_key_root_delegation_batch(
            input(&signing_policy),
            &[DueChainKeyTemplate {
                template: template(issuer, 60_000_000_000),
            }],
            10,
        )
        .expect("batch should build");
        batch.status = ChainKeyRootDelegationBatchStatus::Signed;
        batch.signature = Some(sign_header(&batch.header));
        batch.signed_at_ns = Some(2_000);
        let batch_id = batch.batch_id;
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

        let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
            .expect("install planning should succeed")
            .expect("signed batch should produce an install plan");

        assert_eq!(plan.batch_id, batch_id);
        assert_eq!(plan.proofs.len(), 1);
        let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("installing batch should remain stored");
        assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Installing);
        assert_eq!(stored.install_started_at_ns, Some(3_000));

        let proof = &plan.proofs[0];
        assert_eq!(proof.issuer_pid, issuer);
        verify_chain_key_batch_root_proof(
            crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
                cert: &proof.proof.cert,
                root_proof: &proof.proof.root_proof,
                policy: &verifier_policy(&signing_policy),
                now_ns: 3_000,
            },
            |input: ChainKeySignatureVerificationInput<'_>| verify_chain_key_ecdsa_signature(input),
        )
        .expect("materialized install proof should verify");

        assert!(record_chain_key_root_delegation_install_success(
            batch_id,
            proof.issuer_pid,
            proof.cert_hash,
            4_000,
        ));
        let installed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("installed batch should remain stored");
        assert_eq!(
            installed.status,
            ChainKeyRootDelegationBatchStatus::Installed
        );
        assert_eq!(installed.installed_at_ns, Some(4_000));
        assert_eq!(installed.issuers[0].installed_at_ns, Some(4_000));

        let state = AuthStateOps::root_issuer_renewal_state(issuer)
            .expect("issuer renewal state should be updated");
        assert_eq!(state.last_installed_cert_hash, Some(proof.cert_hash));
        assert_eq!(
            state.last_installed_expires_at_ns,
            Some(proof.proof.cert.expires_at_ns)
        );
        assert_eq!(state.last_outcome, RootIssuerRenewalOutcome::Installed);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(
            start_chain_key_root_delegation_batch_install(batch_id, 5_000)
                .expect("installed batch should be ignored"),
            None
        );
    }

    #[test]
    fn chain_key_batch_partial_install_failure_retries_only_remaining_issuer() {
        let signing_policy = signing_policy();
        let issuer_a = p(64);
        let issuer_b = p(65);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_a));
        AuthStateOps::upsert_root_issuer_policy(policy(issuer_b));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_a, 60_000_000_000));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer_b, 60_000_000_000));
        let mut batch = build_chain_key_root_delegation_batch(
            input(&signing_policy),
            &[
                DueChainKeyTemplate {
                    template: template(issuer_a, 60_000_000_000),
                },
                DueChainKeyTemplate {
                    template: template(issuer_b, 60_000_000_000),
                },
            ],
            10,
        )
        .expect("batch should build");
        batch.status = ChainKeyRootDelegationBatchStatus::Signed;
        batch.signature = Some(sign_header(&batch.header));
        batch.signed_at_ns = Some(2_000);
        let batch_id = batch.batch_id;
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

        let plan = start_chain_key_root_delegation_batch_install(batch_id, 3_000)
            .expect("install planning should succeed")
            .expect("signed batch should produce an install plan");
        assert_eq!(plan.proofs.len(), 2);
        let installed = plan.proofs[0].clone();
        let failed = plan.proofs[1].clone();

        assert!(record_chain_key_root_delegation_install_success(
            batch_id,
            installed.issuer_pid,
            installed.cert_hash,
            4_000,
        ));
        assert!(record_chain_key_root_delegation_install_failure(
            batch_id,
            failed.issuer_pid,
            failed.cert_hash,
            RootDelegationProofInstallOutcome::CallFailed,
        ));

        let partially_installed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("partially installed batch should remain stored");
        assert_eq!(
            partially_installed.status,
            ChainKeyRootDelegationBatchStatus::Installing
        );
        assert_eq!(partially_installed.failure, Some("CallFailed".to_string()));
        assert!(
            partially_installed
                .issuers
                .iter()
                .any(|issuer| issuer.issuer_pid == installed.issuer_pid
                    && issuer.installed_at_ns == Some(4_000))
        );
        assert!(
            partially_installed
                .issuers
                .iter()
                .any(|issuer| issuer.issuer_pid == failed.issuer_pid
                    && issuer.installed_at_ns.is_none()
                    && issuer.last_failure.as_deref() == Some("CallFailed"))
        );

        let retry_plan = start_chain_key_root_delegation_batch_install(batch_id, 5_000)
            .expect("partial retry planning should succeed")
            .expect("failed issuer should remain installable");

        assert_eq!(retry_plan.proofs.len(), 1);
        assert_eq!(retry_plan.proofs[0].issuer_pid, failed.issuer_pid);
        assert!(record_chain_key_root_delegation_install_success(
            batch_id,
            failed.issuer_pid,
            failed.cert_hash,
            6_000,
        ));

        let completed = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("completed batch should remain stored");
        assert_eq!(
            completed.status,
            ChainKeyRootDelegationBatchStatus::Installed
        );
        assert_eq!(completed.installed_at_ns, Some(6_000));
        assert_eq!(completed.failure, None);
        assert!(
            completed
                .issuers
                .iter()
                .all(|issuer| issuer.installed_at_ns.is_some() && issuer.last_failure.is_none())
        );
    }

    #[test]
    fn chain_key_lazy_repair_get_or_create_signs_once_then_reuses_cached_proof() {
        let signing_policy = signing_policy();
        let issuer = p(54);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));
        let mut signer = DynamicMockSigner {
            public_key_calls: 0,
            sign_calls: 0,
        };

        let proof = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
            input(&signing_policy),
            issuer,
            &mut signer,
        ))
        .expect("lazy repair should sign")
        .expect("lazy repair should return a proof");

        assert_eq!(proof.issuer_pid, issuer);
        assert_eq!(signer.public_key_calls, 1);
        assert_eq!(signer.sign_calls, 1);
        verify_chain_key_batch_root_proof(
            crate::ops::auth::delegated::chain_key::VerifyChainKeyBatchRootProofInput {
                cert: &proof.proof.cert,
                root_proof: &proof.proof.root_proof,
                policy: &verifier_policy(&signing_policy),
                now_ns: 1_000,
            },
            |input: ChainKeySignatureVerificationInput<'_>| verify_chain_key_ecdsa_signature(input),
        )
        .expect("lazy repair proof should verify");

        let mut cached_signer = DynamicMockSigner {
            public_key_calls: 0,
            sign_calls: 0,
        };
        let cached = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
            input(&signing_policy),
            issuer,
            &mut cached_signer,
        ))
        .expect("cached lazy repair should succeed")
        .expect("cached lazy repair should return a proof");

        assert_eq!(cached.cert_hash, proof.cert_hash);
        assert_eq!(cached_signer.public_key_calls, 0);
        assert_eq!(cached_signer.sign_calls, 0);
    }

    #[test]
    fn chain_key_lazy_repair_reuses_in_flight_batch_without_extra_signing() {
        let signing_policy = signing_policy();
        let issuer = p(55);
        AuthStateOps::upsert_root_issuer_policy(policy(issuer));
        AuthStateOps::upsert_root_issuer_renewal_template(template(issuer, 60_000_000_000));

        let prepared = prepare_due_chain_key_root_delegation_batch(input(&signing_policy))
            .expect("prepare should build a batch");
        let batch_id = prepared.batch_id.expect("prepare should return a batch id");
        let mut batch = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("prepared batch should be stored");
        batch.status = ChainKeyRootDelegationBatchStatus::Signing;
        AuthStateOps::upsert_chain_key_root_delegation_batch(batch);

        for _ in 0..8 {
            let mut signer = DynamicMockSigner {
                public_key_calls: 0,
                sign_calls: 0,
            };
            let proof = block_on(get_or_create_chain_key_delegation_proof_for_issuer(
                input(&signing_policy),
                issuer,
                &mut signer,
            ))
            .expect("in-flight lazy repair should be retryable later");

            assert_eq!(proof, None);
            assert_eq!(signer.public_key_calls, 0);
            assert_eq!(signer.sign_calls, 0);
        }

        assert_eq!(AuthStateOps::chain_key_root_delegation_batches().len(), 1);
        let stored = AuthStateOps::chain_key_root_delegation_batch(batch_id)
            .expect("in-flight batch should remain stored");
        assert_eq!(stored.status, ChainKeyRootDelegationBatchStatus::Signing);
        assert!(stored.signature.is_none());
    }

    #[test]
    fn chain_key_template_due_respects_refresh_and_template_fingerprint() {
        let issuer = p(60);
        let template = template(issuer, 60_000_000_000);
        let fingerprint = renewal_template_fingerprint(&template);
        let state = RootIssuerRenewalState {
            issuer_pid: issuer,
            template_fingerprint: fingerprint,
            last_installed_cert_hash: Some([1; 32]),
            last_installed_expires_at_ns: Some(200),
            last_installed_refresh_after_ns: Some(100),
            active_attempt_id: None,
            last_outcome: RootIssuerRenewalOutcome::Installed,
            consecutive_failures: 0,
            next_attempt_after_ns: 0,
            updated_at_ns: 10,
        };

        assert!(!chain_key_template_due(99, fingerprint, Some(&state)));
        assert!(chain_key_template_due(100, fingerprint, Some(&state)));
        assert!(chain_key_template_due(99, [9; 32], Some(&state)));

        let mut delayed = state;
        delayed.next_attempt_after_ns = 150;
        assert!(!chain_key_template_due(100, fingerprint, Some(&delayed)));
    }

    #[test]
    fn merkle_witnesses_round_trip_for_odd_leaf_count() {
        let leaves = [[1; 32], [2; 32], [3; 32]];
        let (root, witnesses) = merkle_root_and_witnesses(&leaves).expect("tree should build");

        assert_eq!(witnesses.len(), 3);
        for (leaf, witness) in leaves.into_iter().zip(witnesses) {
            let witness_root = witness.steps.iter().fold(leaf, |current, step| match step {
                ChainKeyBatchWitnessStepV1::LeftSibling(sibling) => {
                    chain_key_batch_node_hash(*sibling, current)
                }
                ChainKeyBatchWitnessStepV1::RightSibling(sibling) => {
                    chain_key_batch_node_hash(current, *sibling)
                }
            });
            assert_eq!(witness_root, root);
        }
    }
}
