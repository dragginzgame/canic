//! Delegated signing DTOs.
//!
//! These types define the data model for delegated signing:
//! a root authority delegates signing capability to shard-local keys,
//! and shards sign tokens that can be verified locally without any
//! directory, registry, or topology lookup.
//!
//! Trust model summary:
//! - The root authority signs DelegationCerts.
//! - Shards sign DelegatedTokens using a delegated key.
//! - Verifiers trust tokens only if the delegation chain is valid.
//! - No canister calls occur during verification.

use crate::dto::prelude::*;

///
/// DelegationCert
///
/// A root-signed certificate that delegates token-signing authority
/// to a specific public key.
///
/// This is the *only* object signed by the root authority canister.
/// Verifiers treat this as the trust anchor for delegated tokens.
///
/// Authority semantics:
/// - The root signature over this struct establishes trust.
/// - `signer_pubkey` is the delegated signing key.
/// - `audiences` and `scopes` bound what the signer is allowed to issue.
/// - `expires_at` limits delegation lifetime.
///
/// Non-authoritative metadata:
/// - `signer_pid` is optional and informational only (audit/debug).
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationCert {
    /// Version of the delegation certificate format.
    /// Allows forward-compatible evolution of fields and semantics.
    pub v: u16,

    /// Stable identifier for the delegated key.
    /// Used as a seed/domain discriminator for root signing
    /// and for key rotation tracking.
    pub key_id: String,

    /// Public key corresponding to the shard’s local signing key.
    /// Tokens must be signed with the private key matching this value.
    pub signer_pubkey: Vec<u8>,

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

    /// Optional PID of the canister holding the signing key.
    ///
    /// This field is NOT authoritative and MUST NOT be used
    /// as a trust decision. It exists only for auditability
    /// and diagnostics.
    pub signer_pid: Option<Principal>,
}

///
/// DelegationProof
///
/// Proof that a DelegationCert was authorized by the root authority.
///
/// This object binds the DelegationCert to a root signature.
/// Verifiers validate `cert_sig_cbor` using the root authority’s
/// public key and a fixed domain separator.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProof {
    /// The delegated certificate describing signer authority.
    pub cert: DelegationCert,

    /// Root authority signature over the CBOR-encoded DelegationCert.
    ///
    /// This signature establishes the delegation chain:
    /// root → signer_pubkey.
    pub cert_sig_cbor: Vec<u8>,
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

    /// Optional tenant identifier.
    /// Informational unless enforced by endpoint policy.
    pub tenant: Option<String>,

    /// Optional pool identifier.
    /// Informational unless enforced by endpoint policy.
    pub pool: Option<String>,
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
/// 4. Verify token signature using delegated public key.
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
    /// produced by the delegated signer’s private key.
    pub signature: Vec<u8>,
}
