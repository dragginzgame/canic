//! Module: ops::auth::types
//!
//! Responsibility: define auth operation input and prepared-result shapes.
//! Does not own: boundary DTO schemas, storage records, or verification policy.
//! Boundary: names cross-helper data passed through auth ops.

use crate::{
    cdk::types::Principal,
    domain::auth::DelegatedAuthNetwork,
    dto::auth::{
        DelegatedRoleGrant, DelegatedToken, DelegationAudience, RoleAttestation, RootKeyPolicyV1,
        RootProofMode,
    },
    ids::BuildNetwork,
    ids::CanisterRole,
    ops::auth::delegated::prepare::PreparedDelegatedToken,
};

///
/// PrepareDelegatedTokenIssuerProofInput
///
/// Auth-ops input for preparing an issuer-local delegated token proof.
///

#[derive(Clone)]
pub struct PrepareDelegatedTokenIssuerProofInput {
    pub subject: Principal,
    pub audience: DelegationAudience,
    pub grants: Vec<DelegatedRoleGrant>,
    pub ttl_ns: u64,
    pub ext: Option<Vec<u8>>,
}

///
/// PrepareRootRoleAttestationInput
///
/// Auth-ops input for preparing a root role attestation proof.
///

pub struct PrepareRootRoleAttestationInput {
    pub operation_id: [u8; 32],
    pub subject: Principal,
    pub role: CanisterRole,
    pub subnet_id: Option<Principal>,
    pub audience: Principal,
    pub ttl_ns: u64,
    pub epoch: u64,
    pub issued_at_ns: u64,
}

///
/// PrepareChainKeyRootDelegationBatchInput
///
/// Runtime input for preparing one due chain-key root delegation batch.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrepareChainKeyRootDelegationBatchInput {
    pub build_network: BuildNetwork,
    pub max_cert_ttl_ns: u64,
    pub min_accepted_proof_epoch: u64,
    pub required_issuer_pid: Option<Principal>,
    pub now_ns: u64,
}

///
/// ChainKeyRootDelegationBatchSweepResult
///
/// Summary of one chain-key root delegation preparation sweep.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChainKeyRootDelegationBatchSweepResult {
    pub batch_id: Option<[u8; 32]>,
    pub prepared_issuers: usize,
    pub skipped_templates: usize,
    pub reused_in_flight: bool,
}

///
/// ChainKeyRootDelegationBatchSigningResult
///
/// Summary of one chain-key root delegation signing step.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChainKeyRootDelegationBatchSigningResult {
    pub batch_id: Option<[u8; 32]>,
    pub signed: bool,
    pub reused_signed: bool,
    pub signing_in_flight: bool,
}

///
/// PreparedRootRoleAttestation
///
/// Prepared role attestation material and retrieval expiry.
///

#[derive(Clone)]
pub struct PreparedRootRoleAttestation {
    pub payload: RoleAttestation,
    pub payload_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

///
/// PreparedDelegatedTokenIssuerProof
///
/// Prepared issuer-local delegated token material and retrieval expiry.
///

pub struct PreparedDelegatedTokenIssuerProof {
    pub prepared: PreparedDelegatedToken,
    pub claims_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
}

///
/// AuthProofVerifierConfig
///
/// Runtime verifier trust-anchor configuration for delegated auth proofs.
///

#[derive(Debug)]
pub struct AuthProofVerifierConfig {
    pub network: DelegatedAuthNetwork,
    pub root_canister_id: Principal,
    pub ic_root_public_key_raw: Vec<u8>,
    pub root_proof_mode: RootProofMode,
    pub chain_key_root: Option<AuthChainKeyRootVerifierConfig>,
}

///
/// AuthChainKeyRootVerifierConfig
///
/// Runtime verifier trust-anchor configuration for chain-key batch root proofs.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthChainKeyRootVerifierConfig {
    pub policy: RootKeyPolicyV1,
    pub allow_test_chain_key: bool,
}

///
/// VerifyDelegatedTokenRuntimeInput
///
/// Auth-ops input for local delegated-token verification.
///

pub struct VerifyDelegatedTokenRuntimeInput<'a> {
    pub token: &'a DelegatedToken,
    pub caller: Principal,
    pub max_cert_ttl_ns: u64,
    pub max_token_ttl_ns: u64,
    pub required_scopes: &'a [String],
    pub now_ns: u64,
}
