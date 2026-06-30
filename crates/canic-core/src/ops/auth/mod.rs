//! Module: ops::auth
//!
//! Responsibility: provide auth proof preparation, verification, and validation ops.
//! Does not own: endpoint authorization, storage schemas, or pure auth policy.
//! Boundary: ops layer between auth workflows, policy, storage, and crypto helpers.

mod attestation;
mod boundary;
#[cfg(any(
    feature = "auth-root-canister-sig-verify",
    feature = "auth-issuer-canister-sig-verify"
))]
mod canister_sig_key;
mod crypto;
mod delegated;
mod delegation;
mod error;
mod issuer_canister_sig;
mod root_canister_sig;
mod token;
mod types;
mod verify;
pub use boundary::DelegatedSessionExpiryClamp;
#[cfg(test)]
pub(crate) use delegated::test_fixtures;
pub use error::{
    AuthExpiryError, AuthOpsError, AuthScopeError, AuthSignatureError, AuthValidationError,
};
pub use types::{
    AuthChainKeyRootVerifierConfig, AuthProofVerifierConfig,
    ChainKeyRootDelegationBatchSigningResult, ChainKeyRootDelegationBatchSweepResult,
    PrepareChainKeyRootDelegationBatchInput, PrepareDelegatedTokenIssuerProofInput,
    PrepareRootRoleAttestationInput, PreparedDelegatedTokenIssuerProof,
    PreparedRootRoleAttestation, VerifyDelegatedTokenRuntimeInput,
};

const ROLE_ATTESTATION_PROOF_HASH_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
pub const AUTH_TIME_SKEW_ALLOWANCE_NS: u64 = 60_000_000_000;

///
/// AuthOps
///
/// Operations-layer facade for auth proof preparation and verification.
///

pub struct AuthOps;
