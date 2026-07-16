//! Module: ops::auth::delegation::chain_key_batch::install
//!
//! Responsibility: materialize issuer payloads and persist batch installation outcomes.
//! Does not own: issuer inter-canister calls, batch preparation, or signing.
//! Boundary: deterministic install planning and result recording for auth workflows.

use super::super::root_issuer_renewal::renewal_template_fingerprint;
use super::ChainKeyRootDelegationBatchInstallPlan;
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::auth::{
        ChainKeyRootSignatureV1, DelegationProof, IcChainKeyBatchSignatureProofV1,
        RootDelegationProofBatchProof, RootProof,
    },
    model::auth::{ChainKeyRootDelegationInstallFailure, RootIssuerRenewalState},
    ops::storage::auth::{
        AuthStateOps, ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchIssuer,
        ChainKeyRootDelegationBatchStatus,
    },
};

pub(super) fn signed_chain_key_delegation_proof_for_issuer(
    issuer_pid: Principal,
    now_ns: u64,
    registry_epoch: u64,
    registry_hash: [u8; 32],
) -> Option<RootDelegationProofBatchProof> {
    let mut batches = AuthStateOps::chain_key_root_delegation_batches()
        .into_iter()
        .filter(|batch| now_ns < batch.header.expires_at_ns)
        .filter(|batch| super::batch_matches_registry(batch, registry_epoch, registry_hash))
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

pub(super) fn materialize_chain_key_delegation_proof(
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

pub(in crate::ops::auth) fn start_next_chain_key_root_delegation_batch_install(
    now_ns: u64,
) -> Result<Option<ChainKeyRootDelegationBatchInstallPlan>, InternalError> {
    AuthStateOps::prune_chain_key_root_delegation_batches(now_ns);
    let Some(batch) = next_chain_key_batch_for_install(now_ns) else {
        return Ok(None);
    };
    start_chain_key_root_delegation_batch_install(batch.batch_id, now_ns)
}

pub(super) fn start_chain_key_root_delegation_batch_install(
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
    failure: ChainKeyRootDelegationInstallFailure,
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

    let reason = format!("{failure:?}");
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
        next_attempt_after_ns: issuer.refresh_after_ns,
        updated_at_ns: now_ns,
    });
}
