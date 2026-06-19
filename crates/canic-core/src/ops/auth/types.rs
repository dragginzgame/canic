//! Module: ops::auth::types
//!
//! Responsibility: define auth operation input and prepared-result shapes.
//! Does not own: boundary DTO schemas, storage records, or verification policy.
//! Boundary: names cross-helper data passed through auth ops.

use crate::{
    cdk::types::Principal,
    domain::auth::DelegatedAuthNetwork,
    dto::auth::{
        DelegatedRoleGrant, DelegatedToken, DelegationAudience, DelegationCert, RoleAttestation,
    },
    ids::CanisterRole,
    ops::auth::delegated::prepare::PreparedDelegatedToken,
};

///
/// PrepareDelegatedTokenIssuerProofInput
///
/// Auth-ops input for preparing an issuer-local delegated token proof.
///

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
/// PreparedRootDelegationProof
///
/// Prepared root delegation proof material and retrieval expiry.
///

#[derive(Clone)]
pub struct PreparedRootDelegationProof {
    pub cert: DelegationCert,
    pub cert_hash: [u8; 32],
    pub retrieval_expires_at_ns: u64,
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
/// Runtime verifier trust-anchor configuration for canister-signature proofs.
///

#[derive(Debug)]
pub struct AuthProofVerifierConfig {
    pub network: DelegatedAuthNetwork,
    pub root_canister_id: Principal,
    pub ic_root_public_key_raw: Vec<u8>,
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
