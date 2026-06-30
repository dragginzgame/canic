//! Test fixtures for delegated-auth proof shapes.

use crate::{
    cdk::types::Principal,
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1,
        ChainKeyKeyId, ChainKeyRootSignatureV1, DelegatedRoleGrant, DelegationAudience,
        IcChainKeyBatchSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding, RootProof,
    },
    ids::CanisterRole,
    ops::auth::{
        delegated::canonical::chain_key_delegation_cert_hash,
        issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
    },
};

pub(crate) fn chain_key_root_proof(byte: u8) -> RootProof {
    let root_canister_id = principal(byte);
    let issuer_canister_id = principal(byte.saturating_add(1));
    let issuer_proof_algorithm = IssuerProofAlgorithm::IcCanisterSignatureV1;
    let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
        seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
    };
    let delegation_cert = ChainKeyDelegationCertV1 {
        root_canister_id,
        issuer_canister_id,
        proof_epoch: u64::from(byte) + 1,
        issuer_proof_algorithm,
        issuer_proof_binding_hash: [byte; 32],
        issuer_proof_binding,
        max_token_ttl_ns: 60,
        audience: DelegationAudience::Project(format!("test-{byte}")),
        grants: vec![DelegatedRoleGrant {
            target: CanisterRole::owned(format!("role-{byte}")),
            scopes: vec!["read".to_string()],
        }],
        not_before_ns: 10,
        expires_at_ns: 70,
        registry_epoch: u64::from(byte) + 2,
        registry_hash: [byte.saturating_add(2); 32],
    };
    let leaf_hash =
        chain_key_delegation_cert_hash(&delegation_cert).expect("fixture cert should hash");
    let header = ChainKeyBatchHeaderV1 {
        schema_version: 1,
        root_canister_id,
        batch_id: [byte.saturating_add(3); 32],
        proof_epoch: delegation_cert.proof_epoch,
        registry_epoch: delegation_cert.registry_epoch,
        registry_hash: delegation_cert.registry_hash,
        tree_root: leaf_hash,
        not_before_ns: delegation_cert.not_before_ns,
        expires_at_ns: delegation_cert.expires_at_ns,
        algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
        key_id: ChainKeyKeyId {
            name: format!("test-key-{byte}"),
        },
        derivation_path_hash: [byte.saturating_add(4); 32],
        key_version: 1,
    };
    RootProof::IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1 {
        header,
        delegation_cert,
        issuer_witness: ChainKeyBatchWitnessV1 { steps: Vec::new() },
        signature: ChainKeyRootSignatureV1 {
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: format!("test-key-{byte}"),
            },
            derivation_path: vec![vec![byte]],
            public_key: vec![byte; 33],
            signature: vec![byte; 64],
        },
    })
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}
