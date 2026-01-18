//! Delegated signing DTOs.
//!
//! These types define the data model for delegated signing using
//! IC canister signatures only:
//! a root authority delegates signing capability to signer canisters,
//! and signers issue tokens that can be verified locally without any
//! directory, registry, or topology lookup.
//!
//! Trust model summary:
//! - The root authority signs DelegationCerts (IC canister signature).
//! - Signer canisters sign DelegatedTokens (IC canister signature).
//! - Verifiers trust tokens only if the delegation chain is valid.
//! - No canister calls occur during verification.

use crate::dto::prelude::*;

///
/// DelegationCert
///
/// A root-signed certificate that delegates token-signing authority
/// to a signer canister principal.
///
/// This is the *only* object signed by the root authority canister.
/// Verifiers treat this as the trust anchor for delegated tokens.
///
/// Authority semantics:
/// - The root signature over this struct establishes trust.
/// - `signer_pid` is the delegated signer canister.
/// - `audiences` and `scopes` bound what the signer is allowed to issue.
/// - `expires_at` limits delegation lifetime.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationCert {
    /// Version of the delegation certificate format.
    /// Allows forward-compatible evolution of fields and semantics.
    pub v: u16,

    /// Delegated signer canister principal.
    /// Tokens must be signed by this canister.
    pub signer_pid: Principal,

    /// Audiences the delegated signer is allowed to issue tokens for.
    /// Token claims.aud must be a member of this set.
    pub audiences: Vec<String>,

    /// Scopes the delegated signer is allowed to assert.
    /// Token claims.scopes must be a subset of this set.
    pub scopes: Vec<String>,

    /// Time (seconds since epoch) when this delegation was issued.
    pub issued_at: u64,

    /// Absolute expiration time of this delegation.
    /// Tokens must not outlive the certificate.
    pub expires_at: u64,
}

///
/// DelegationProof
///
/// Proof that a DelegationCert was authorized by the root authority.
///
/// This object binds the DelegationCert to a root signature.
/// Verifiers validate `cert_sig` using the root authority’s
/// public key and a fixed domain separator.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProof {
    /// The delegated certificate describing signer authority.
    pub cert: DelegationCert,

    /// Root authority signature over the DelegationCert hash.
    ///
    /// This signature establishes the delegation chain:
    /// root → signer_pid.
    pub cert_sig: Vec<u8>,
}

///
/// DelegatedTokenClaims
///
/// Claims asserted by a delegated signer.
///
/// These claims are signed by the delegated key and are validated
/// against both the DelegationCert and local policy.
///
/// Trust semantics:
/// - All fields are untrusted until signature verification succeeds.
/// - Authorization derives from *both* claims and delegation.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegatedTokenClaims {
    /// Subject of the token (e.g. user principal).
    pub sub: Principal,

    /// Intended audience of the token.
    /// Must match one of DelegationCert.audiences.
    pub aud: String,

    /// Scopes asserted by this token.
    /// Must be a subset of DelegationCert.scopes.
    pub scopes: Vec<String>,

    /// Issued-at timestamp.
    pub iat: u64,

    /// Expiration timestamp.
    /// Must be <= DelegationCert.expires_at.
    pub exp: u64,

    /// Optional nonce for replay protection or correlation.
    pub nonce: Option<Vec<u8>>,
}

///
/// DelegatedToken
///
/// A signed authorization token issued by a delegated signer.
///
/// Verification steps (high level):
/// 1. Validate token version.
/// 2. Verify delegation certificate root signature.
/// 3. Validate time bounds and scope constraints.
/// 4. Verify token signature using the signer canister.
///
/// No directory, registry, or topology lookup is required.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegatedToken {
    /// Version of the delegated token format.
    pub v: u16,

    /// Claims asserted by the signer.
    pub claims: DelegatedTokenClaims,

    /// Delegation proof binding signer authority to root trust.
    pub proof: DelegationProof,

    /// Signature over canonicalized claims and delegation hash,
    /// produced by the signer canister.
    pub token_sig: Vec<u8>,
}
