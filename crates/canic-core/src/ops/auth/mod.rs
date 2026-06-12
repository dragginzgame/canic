mod attestation;
mod boundary;
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
pub use error::{
    AuthExpiryError, AuthOpsError, AuthScopeError, AuthSignatureError, AuthValidationError,
};
pub use types::{
    DelegatedTokenVerifierConfig, PreparedDelegatedTokenIssuerProof, PreparedRootDelegationProof,
    PreparedRootRoleAttestation, SignDelegatedTokenInput, SignDelegationProofInput,
    SignRoleAttestationInput, VerifyDelegatedTokenRuntimeInput,
};

const ROLE_ATTESTATION_PROOF_HASH_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
pub const AUTH_TIME_SKEW_ALLOWANCE_NS: u64 = 60_000_000_000;

///
/// AuthOps
///

pub struct AuthOps;
