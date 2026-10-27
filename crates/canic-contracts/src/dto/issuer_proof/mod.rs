//! Native issuer proof transport. Cryptographic and stored encodings remain owned by auth.
use crate::dto::auth::{
    ChainKeyAlgorithm, ChainKeyBatchWitnessV1, ChainKeyKeyId, ChainKeyRootSignatureV1,
    DelegatedRoleGrant, DelegationAudience, IssuerProofAlgorithm, IssuerProofBinding,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native ChainKeyBatchHeaderV1 transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChainKeyBatchHeaderV1 {
    pub schema_version: u16,
    pub root_canister_id: Principal,
    pub batch_id: [u8; 32],
    pub proof_epoch: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
    pub tree_root: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path_hash: [u8; 32],
    pub key_version: u64,
}

/// Native IcChainKeyBatchSignatureProofV1 transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IcChainKeyBatchSignatureProofV1 {
    pub header: ChainKeyBatchHeaderV1,
    pub delegation_cert: ChainKeyDelegationCertV1,
    pub issuer_witness: ChainKeyBatchWitnessV1,
    pub signature: ChainKeyRootSignatureV1,
}

/// Native ChainKeyDelegationCertV1 transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChainKeyDelegationCertV1 {
    pub root_canister_id: Principal,
    pub issuer_canister_id: Principal,
    pub proof_epoch: u64,
    pub issuer_proof_algorithm: IssuerProofAlgorithm,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBinding,
    pub max_token_ttl_ns: u64,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub registry_epoch: u64,
    pub registry_hash: [u8; 32],
}

/// Native DelegationCert transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DelegationCert {
    pub root_pid: Principal,
    pub issuer_pid: Principal,
    pub issuer_proof_alg: IssuerProofAlgorithm,
    pub issuer_proof_binding_hash: [u8; 32],
    pub issuer_proof_binding: IssuerProofBinding,
    pub issued_at_ns: u64,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub max_token_ttl_ns: u64,
    pub aud: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
}

/// Native DelegationProof transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DelegationProof {
    pub cert: DelegationCert,
    pub root_proof: RootProof,
}

/// Native ActiveDelegationProof transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActiveDelegationProof {
    pub proof: DelegationProof,
    pub cert_hash: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub refresh_after_ns: u64,
    pub installed_at_ns: u64,
    pub installed_by: Principal,
}

/// Native InstallActiveDelegationProofRequest transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct InstallActiveDelegationProofRequest {
    pub proof: DelegationProof,
}

/// Native InstallActiveDelegationProofResponse transport.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct InstallActiveDelegationProofResponse {
    pub active_proof: ActiveDelegationProof,
}

/// Native Root proof; threshold signing semantics remain unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum RootProof {
    IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1),
}

/// Public issuer-local proof evidence; no bearer credential or secret material.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActiveDelegationProofStatusResponse {
    pub status: crate::dto::auth::ActiveDelegationProofStatus,
    pub root_pid: Option<Principal>,
    pub issuer_pid: Option<Principal>,
    pub cert_hash: Option<[u8; 32]>,
    pub expires_at_ns: Option<u64>,
    pub refresh_after_ns: Option<u64>,
}

#[cfg(test)]
mod tests;
