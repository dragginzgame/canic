#[cfg(test)]
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{AttestationKey, RoleAttestation, SignedRoleAttestation},
    ops::storage::auth::DelegationStateOps,
    workflow::prelude::CanisterRole,
};

mod attestation;
mod crypto;
mod delegation;
mod error;
mod keys;
mod token;
mod types;
mod verify;
pub use error::{
    DelegatedTokenOpsError, DelegationExpiryError, DelegationScopeError, DelegationSignatureError,
    DelegationValidationError,
};
pub use types::VerifiedDelegatedToken;

const DERIVATION_NAMESPACE: &[u8] = b"canic";
const ROOT_PATH_SEGMENT: &[u8] = b"root";
const SHARD_PATH_SEGMENT: &[u8] = b"shard";
const ATTESTATION_PATH_SEGMENT: &[u8] = b"attestation";
const CERT_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATION_CERT_V1";
const TOKEN_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATED_TOKEN_V1";
const ROLE_ATTESTATION_SIGNING_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
const ROLE_ATTESTATION_KEY_ID_V1: u32 = 1;

///
/// DelegatedTokenOps
///

pub struct DelegatedTokenOps;

#[cfg(test)]
fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> Result<(), DelegatedTokenOpsError> {
    verify::verify_role_attestation_claims(
        payload,
        caller,
        self_pid,
        verifier_subnet,
        now_secs,
        min_accepted_epoch,
    )
}

#[cfg(test)]
fn attestation_keys_sorted() -> Vec<AttestationKey> {
    keys::attestation_keys_sorted()
}

#[cfg(test)]
fn root_derivation_path() -> Vec<Vec<u8>> {
    keys::root_derivation_path()
}

#[cfg(test)]
fn attestation_derivation_path() -> Vec<Vec<u8>> {
    keys::attestation_derivation_path()
}

#[cfg(test)]
fn role_attestation_hash(attestation: &RoleAttestation) -> Result<[u8; 32], InternalError> {
    crypto::role_attestation_hash(attestation)
}

#[cfg(test)]
mod tests;
