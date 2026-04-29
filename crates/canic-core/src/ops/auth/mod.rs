use crate::ops::ic::IcOps;

mod attestation;
mod boundary;
mod crypto;
mod delegated;
mod delegation;
mod error;
mod keys;
mod token;
mod types;
mod verify;
pub use boundary::DelegatedSessionExpiryClamp;
pub use error::{
    DelegatedTokenOpsError, DelegationExpiryError, DelegationScopeError, DelegationSignatureError,
    DelegationValidationError,
};
pub use types::{
    SignDelegatedTokenInput, SignDelegationProofInput, VerifyDelegatedTokenRuntimeInput,
};

const DERIVATION_NAMESPACE: &[u8] = b"canic";
const ROOT_PATH_SEGMENT: &[u8] = b"root";
const SHARD_PATH_SEGMENT: &[u8] = b"shard";
const ATTESTATION_PATH_SEGMENT: &[u8] = b"attestation";
const ROLE_ATTESTATION_SIGNING_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
const ROLE_ATTESTATION_KEY_ID_V1: u32 = 1;

///
/// DelegatedTokenOps
///

pub struct DelegatedTokenOps;

impl DelegatedTokenOps {
    // Publish the root delegation public key into cascaded subnet state.
    pub async fn publish_root_delegated_key_material() -> Result<(), crate::InternalError> {
        let root_pid = IcOps::canister_self();
        let delegated_key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_root_public_key_published(&delegated_key_name, root_pid).await
    }

    // Publish root auth material and warm local root-owned auth keys once.
    pub async fn publish_root_auth_material() -> Result<(), crate::InternalError> {
        let root_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();

        Self::publish_root_delegated_key_material().await?;

        let attestation_key_name = keys::attestation_key_name()?;
        keys::ensure_attestation_key_cached(&attestation_key_name, root_pid, now_secs).await?;

        Ok(())
    }
}
