//! Module: dto::auth::proof
//!
//! Responsibility: delegated root proof, issuer proof, and active proof DTOs.
//! Does not own: proof verification, key validation, or storage mapping.
//! Boundary: passive proof contracts carried by delegated tokens and issuer installs.

use super::{DelegatedRoleGrant, DelegationAudience};
use crate::{dto::prelude::*, ids::BuildNetwork};

//
// RootProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootProof {
    IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1),
}

//
// RootProofMode
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootProofMode {
    ChainKeyBatch,
}

//
// IssuerProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProof {
    IcCanisterSignatureV1(IcCanisterSignatureProofV1),
}

//
// IcCanisterSignatureProofV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcCanisterSignatureProofV1 {
    pub signature_cbor: Vec<u8>,
    pub public_key_der: Vec<u8>,
}

//
// ChainKeyAlgorithm
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChainKeyAlgorithm {
    EcdsaSecp256k1,
}

//
// ChainKeyKeyId
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyKeyId {
    pub name: String,
}

//
// RootKeyPolicyV1
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootKeyPolicyV1 {
    pub root_canister_id: Principal,
    pub proof_mode: RootProofMode,
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path_hash: [u8; 32],
    pub public_key: Vec<u8>,
    pub key_version: u64,
    pub min_accepted_key_version: u64,
    pub min_accepted_proof_epoch: u64,
    pub min_accepted_registry_epoch: u64,
    pub max_revocation_latency_ns: u64,
    pub valid_from_ns: u64,
    pub accept_until_ns: u64,
    pub build_network: BuildNetwork,
}

//
// DelegatedAuthRegistrySnapshotV1
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedAuthRegistrySnapshotV1 {
    pub schema_version: u16,
    pub root_canister_id: Principal,
    pub registry_epoch: u64,
    pub proof_mode: RootProofMode,
    pub root_key_policy_hash: [u8; 32],
    pub issuer_policies: Vec<DelegatedAuthIssuerPolicySnapshotV1>,
}

//
// DelegatedAuthIssuerPolicySnapshotV1
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedAuthIssuerPolicySnapshotV1 {
    pub issuer_canister_id: Principal,
    pub enabled: bool,
    pub preferred_proof_mode: RootProofMode,
    pub allowed_audiences: Vec<DelegationAudience>,
    pub allowed_grants: Vec<DelegatedRoleGrant>,
    pub max_root_proof_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
    pub issuer_proof_algorithm: IssuerProofAlgorithm,
    pub issuer_proof_binding_hash: [u8; 32],
    pub renewal_template_hash: [u8; 32],
}

//
// IcChainKeyBatchSignatureProofV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcChainKeyBatchSignatureProofV1 {
    pub header: ChainKeyBatchHeaderV1,
    pub delegation_cert: ChainKeyDelegationCertV1,
    pub issuer_witness: ChainKeyBatchWitnessV1,
    pub signature: ChainKeyRootSignatureV1,
}

//
// ChainKeyBatchHeaderV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

//
// ChainKeyDelegationCertV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

//
// ChainKeyRootSignatureV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyRootSignatureV1 {
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path: Vec<Vec<u8>>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

//
// ChainKeyBatchWitnessV1
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChainKeyBatchWitnessV1 {
    pub steps: Vec<ChainKeyBatchWitnessStepV1>,
}

//
// ChainKeyBatchWitnessStepV1
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChainKeyBatchWitnessStepV1 {
    LeftSibling([u8; 32]),
    RightSibling([u8; 32]),
}

//
// IssuerProofAlgorithm
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProofAlgorithm {
    IcCanisterSignatureV1,
}

//
// IssuerProofBinding
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IssuerProofBinding {
    IcCanisterSignatureV1 { seed_hash: [u8; 32] },
}

//
// DelegationCert
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

//
// DelegationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegationProof {
    pub cert: DelegationCert,
    pub root_proof: RootProof,
}

//
// ActiveDelegationProof
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDelegationProof {
    pub proof: DelegationProof,
    pub cert_hash: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub refresh_after_ns: u64,
    pub installed_at_ns: u64,
    pub installed_by: Principal,
}

//
// InstallActiveDelegationProofRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallActiveDelegationProofRequest {
    pub proof: DelegationProof,
}

//
// InstallActiveDelegationProofResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallActiveDelegationProofResponse {
    pub active_proof: ActiveDelegationProof,
}

//
// ActiveDelegationProofStatus
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActiveDelegationProofStatus {
    Missing,
    Valid,
    RefreshNeeded,
    Expired,
}

//
// ActiveDelegationProofStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActiveDelegationProofStatusResponse {
    pub status: ActiveDelegationProofStatus,
    pub root_pid: Option<Principal>,
    pub issuer_pid: Option<Principal>,
    pub cert_hash: Option<[u8; 32]>,
    pub expires_at_ns: Option<u64>,
    pub refresh_after_ns: Option<u64>,
}
