//! Module: ops::auth::delegation::chain_key_batch::batch_id
//!
//! Responsibility: derive deterministic chain-key root delegation batch ids.
//! Does not own: canonical auth DTO encoders or persisted batch state.
//! Boundary: private helper for the chain-key batch builder.

use crate::{cdk::types::Principal, dto::auth::ChainKeyAlgorithm};
use sha2::{Digest, Sha256};

const CHAIN_KEY_BATCH_ID_DOMAIN: &[u8] = b"CANIC_ROOT_DELEGATION_CHAIN_KEY_BATCH_ID_V1";

pub(super) struct ChainKeyBatchIdInput<'a> {
    pub(super) root_canister_id: Principal,
    pub(super) proof_epoch: u64,
    pub(super) registry_epoch: u64,
    pub(super) registry_hash: [u8; 32],
    pub(super) tree_root: [u8; 32],
    pub(super) not_before_ns: u64,
    pub(super) expires_at_ns: u64,
    pub(super) algorithm: ChainKeyAlgorithm,
    pub(super) key_id_name: &'a str,
    pub(super) derivation_path_hash: [u8; 32],
    pub(super) key_version: u64,
}

pub(super) fn chain_key_batch_id(input: ChainKeyBatchIdInput<'_>) -> [u8; 32] {
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
