//! Module: ops::auth::delegation::chain_key_batch::install
//!
//! Responsibility: materialize issuer install payloads from signed chain-key batches.
//! Does not own: issuer inter-canister calls or install-result persistence.
//! Boundary: private helper for root proof install planning and lazy repair.

use crate::{
    cdk::types::Principal,
    dto::auth::{
        ChainKeyRootSignatureV1, DelegationProof, IcChainKeyBatchSignatureProofV1,
        RootDelegationProofBatchProof, RootProof,
    },
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
