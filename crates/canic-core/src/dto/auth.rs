//! Delegated signing DTOs.
//!
//! These types define the **data model and trust boundaries** for delegated
//! authorization using **IC canister signatures only**.
//!
//! High-level design:
//! - A single root authority canister is the trust anchor.
//! - The root delegates signing authority to signer canisters via certificates.
//! - Signer canisters issue authorization tokens.
//! - Tokens are verified **locally** with no directory, registry, or topology calls.
//!
//! This model enables:
//! - Offline / deterministic verification
//! - No runtime dependency on external canisters
//! - Clear, auditable trust chains
//!
//! Any change to these structures is **security-sensitive** and must be
//! evaluated against the trust model below.

use crate::dto::prelude::*;

/// ---------------------------------------------------------------------------
/// Trust model summary
/// ---------------------------------------------------------------------------
///
/// - The **root authority canister** is the only long-term trust anchor.
/// - The root signs `DelegationCert` objects using IC canister signatures.
/// - A `DelegationCert` grants limited signing authority to a signer canister.
/// - Signer canisters sign `DelegatedToken` objects.
/// - Verifiers trust a token **only if**:
///     - The delegation certificate is root-signed
///     - The token signature matches the delegated signer
///     - All temporal, scope, and audience constraints hold
///
/// No canister calls occur during verification.
/// All trust is established cryptographically.
///
/// ---------------------------------------------------------------------------

///
/// DelegationCert
///
/// A root-signed certificate that delegates token-signing authority
/// to a signer canister.
///
/// WHY THIS EXISTS
/// ----------------
/// This is the *only* object signed by the root authority canister.
/// It is the **trust anchor** for all delegated tokens.
///
/// If this object is valid and trusted:
/// - The signer canister is authorized to issue tokens
/// - But *only* within the audiences, scopes, and lifetime specified here
///
/// Anything not explicitly allowed here is forbidden.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationCert {
    /// Version of the delegation certificate format.
    ///
    /// WHY:
    /// - Allows forward-compatible evolution of fields and semantics
    /// - Prevents silent misinterpretation during upgrades
    pub v: u16,

    /// Principal of the delegated signer canister.
    ///
    /// Tokens must be signed by this canister.
    /// No other signer is valid under this certificate.
    pub signer_pid: Principal,

    /// Audiences the delegated signer is allowed to issue tokens for.
    ///
    /// Token `claims.aud` MUST be a member of this set.
    /// This prevents a signer from issuing tokens for unrelated services.
    pub audiences: Vec<String>,

    /// Scopes the delegated signer is allowed to assert.
    ///
    /// Token `claims.scopes` MUST be a subset of this set.
    /// This ensures least-privilege delegation.
    pub scopes: Vec<String>,

    /// Time (seconds since epoch) when this delegation was issued.
    ///
    /// Used for auditing and temporal validation.
    pub issued_at: u64,

    /// Absolute expiration time of this delegation.
    ///
    /// Tokens MUST NOT outlive this value, regardless of token TTL.
    /// Rotation invalidates all tokens bound to an expired certificate.
    pub expires_at: u64,
}

///
/// DelegationProof
///
/// Cryptographic proof that a `DelegationCert` was authorized by the root.
///
/// WHY THIS EXISTS
/// ----------------
/// The `DelegationCert` alone is just data.
/// This struct binds it to a **root canister signature**, establishing
/// a verifiable trust chain:
///
///     root authority → signer canister
///
/// Verifiers validate `cert_sig` using:
/// - the root authority’s public key
/// - a fixed domain separator
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProof {
    /// The delegated certificate describing signer authority.
    pub cert: DelegationCert,

    /// Root authority signature over the DelegationCert hash.
    ///
    /// This signature:
    /// - Authenticates the certificate
    /// - Prevents forgery or tampering
    /// - Anchors the delegation chain
    pub cert_sig: Vec<u8>,
}

///
/// DelegatedTokenClaims
///
/// Claims asserted by a delegated signer.
///
/// IMPORTANT:
/// -----------
/// All fields in this struct are **untrusted input** until:
/// - the delegation proof is verified
/// - the token signature is verified
///
/// Authorization derives from *both* the claims AND the delegation.
/// Claims alone are never sufficient.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegatedTokenClaims {
    /// Subject of the token (e.g. user principal).
    pub sub: Principal,

    /// Intended audience of the token.
    ///
    /// MUST match one of `DelegationCert.audiences`.
    /// This prevents cross-service token reuse.
    pub aud: String,

    /// Scopes asserted by this token.
    ///
    /// MUST be a subset of `DelegationCert.scopes`.
    /// The signer cannot exceed delegated authority.
    pub scopes: Vec<String>,

    /// Issued-at timestamp.
    ///
    /// Used to enforce token freshness and TTL constraints.
    pub iat: u64,

    /// Expiration timestamp.
    ///
    /// MUST be:
    /// - >= iat
    /// - <= DelegationCert.expires_at
    pub exp: u64,

    /// Optional nonce for replay protection or correlation.
    ///
    /// Semantics are intentionally undefined at this layer.
    /// Consumers may use it for replay detection or tracing.
    pub nonce: Option<Vec<u8>>,
}

///
/// DelegatedToken
///
/// A signed authorization token issued by a delegated signer.
///
/// Verification steps (normative):
/// 1. Validate token version.
/// 2. Verify the root signature on the delegation certificate.
/// 3. Validate time bounds and scope/audience constraints.
/// 4. Verify the token signature using the signer canister.
///
/// Design constraints:
/// - No directory or registry lookup is required.
/// - No topology or environment inspection is required.
/// - All verification is local and deterministic.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegatedToken {
    /// Version of the delegated token format.
    ///
    /// Allows format evolution without ambiguity.
    pub v: u16,

    /// Claims asserted by the signer.
    pub claims: DelegatedTokenClaims,

    /// Delegation proof binding signer authority to root trust.
    pub proof: DelegationProof,

    /// Signature over canonicalized claims and delegation hash.
    ///
    /// Produced by the delegated signer canister.
    pub token_sig: Vec<u8>,
}

///
/// DelegationAdminCommand
///
/// Administrative commands for managing delegation rotation.
///
/// These commands are expected to be root-authorized and are
/// intentionally narrow in scope.
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum DelegationAdminCommand {
    /// Start periodic delegation rotation.
    ///
    /// `interval_secs` defines how frequently new certificates are issued.
    StartRotation { interval_secs: u64 },

    /// Stop delegation rotation.
    StopRotation,
}

///
/// DelegationAdminResponse
///
/// Result of executing a delegation admin command.
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum DelegationAdminResponse {
    RotationStarted,
    RotationAlreadyRunning,
    RotationStopped,
    RotationNotRunning,
}
