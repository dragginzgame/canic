//! Root-managed portable credential enrollment for one exact receiver installation.

use crate::{
    dto::prelude::*,
    ids::{CanisterRole, FleetKey},
};

#[cfg(test)]
mod tests;

/// Exact authority and audience authenticated independently of a credential.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationEnrollmentBinding {
    pub fleet: FleetKey,
    pub root: Principal,
    pub root_installation: [u8; 32],
    pub receiver: Principal,
    pub receiver_installation: [u8; 32],
    pub role: CanisterRole,
}

/// Verification mechanism approved by Root, never selected by submitted claims.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ApplicationEnrollmentKey {
    Certified,
    Ed25519([u8; 32]),
}

/// Root-approved key and scope ceiling for the selected receiver role.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationEnrollmentEntry {
    pub issuer: Principal,
    pub key_id: [u8; 32],
    pub key: ApplicationEnrollmentKey,
    pub scopes: Vec<String>,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub approval_digest: [u8; 32],
}

/// Complete current authority response to an authenticated receiver pull.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationEnrollmentSnapshot {
    pub binding: ApplicationEnrollmentBinding,
    pub challenge: [u8; 32],
    pub revision: u64,
    pub generation: u64,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub entries: Vec<ApplicationEnrollmentEntry>,
    pub revoked_keys: Vec<[u8; 32]>,
}

/// Request bound to the receiver's current boot or refresh attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationEnrollmentRequest {
    pub binding: ApplicationEnrollmentBinding,
    pub challenge: [u8; 32],
}
