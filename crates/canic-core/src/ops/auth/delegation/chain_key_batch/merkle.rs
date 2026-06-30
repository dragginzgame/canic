//! Module: ops::auth::delegation::chain_key_batch::merkle
//!
//! Responsibility: build deterministic Merkle roots and issuer witnesses.
//! Does not own: batch preparation policy, signing, or persisted state.
//! Boundary: private helper for chain-key batch proof material.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1,
        DelegationCert,
    },
};
use sha2::{Digest, Sha256};

pub(super) struct ChainKeyBatchLeaf {
    pub(super) delegation_cert: DelegationCert,
    pub(super) chain_key_delegation_cert: ChainKeyDelegationCertV1,
    pub(super) cert_hash: [u8; 32],
    pub(super) leaf_hash: [u8; 32],
    pub(super) refresh_after_ns: u64,
}

pub(super) fn reject_duplicate_chain_key_issuers(
    leaves: &[ChainKeyBatchLeaf],
) -> Result<(), InternalError> {
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

pub(super) fn merkle_root_and_witnesses(
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

pub(super) fn chain_key_batch_node_hash(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([1]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}
