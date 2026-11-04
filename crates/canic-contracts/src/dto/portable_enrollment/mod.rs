//! Passive portable enrollment exchange. Decoding never installs trust.

use crate::ids::FleetKey;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Root derives the receiver from its authenticated caller, never a request field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableEnrollmentRequest {
    pub boot_challenge: [u8; 32],
}

/// The Root-approved proof mechanism; Ed25519 carries no implicit custody claim.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PortableIssuerKey {
    Certified { certificate_hash: [u8; 32] },
    Ed25519 { public_key: [u8; 32] },
}

/// Role-local grants; canonical labels/order are checked by Auth at the boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableEnrollmentGrant {
    pub role: String,
    pub scopes: Vec<String>,
}

/// One issuer approved by governance and authenticated registration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuerEnrollment {
    pub issuer: Principal,
    pub key: PortableIssuerKey,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub grants: Vec<PortableEnrollmentGrant>,
    pub deployment_approval_digest: [u8; 32],
    pub deployment_approval_expires_at_ns: u64,
}

/// Complete domain snapshot, including terminal retirements, bound to one pull.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableEnrollmentSnapshot {
    pub authority: Principal,
    pub receiver: Principal,
    pub fleet: FleetKey,
    pub boot_challenge: [u8; 32],
    pub revision: u64,
    pub generation: u64,
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
    pub entries: Vec<PortableIssuerEnrollment>,
    pub retired_keys: Vec<[u8; 32]>,
}

/// Explicit governance choice; an ordinary subnet never implies TEE assurance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PortableIssuerCustody {
    TrustedVolatileLocal,
    TeeRequired,
}

/// Controller-approved public authority, inactive until authenticated key possession.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuerApproval {
    pub issuer: Principal,
    pub public_key: [u8; 32],
    pub custody: PortableIssuerCustody,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub grants: Vec<PortableEnrollmentGrant>,
    pub governance_digest: [u8; 32],
    pub governance_expires_at_ns: u64,
}

/// Exact operator intent; a retry must retain both this ID and every approval field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PreparePortableIssuer {
    pub operation_id: [u8; 32],
    pub approval: PortableIssuerApproval,
}

/// Root-issued signing payload, bound to its domain, approval and finite deadline.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuerChallenge {
    pub authority: Principal,
    pub fleet: FleetKey,
    pub issuer: Principal,
    pub public_key: [u8; 32],
    pub operation_id: [u8; 32],
    pub approval_digest: [u8; 32],
    pub nonce: [u8; 32],
    pub issued_at_ns: u64,
    pub expires_at_ns: u64,
}

/// Actual issuer caller supplies only the exact operation and possession signature.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RegisterPortableIssuer {
    pub operation_id: [u8; 32],
    pub signature: Vec<u8>,
}

/// Lookup selects only the actual caller's original registration operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuerRegistrationLookup {
    pub operation_id: [u8; 32],
}

/// Controller instruction to cancel an issuer's work and retire its issued keys.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RetirePortableIssuer {
    pub issuer: Principal,
}

/// Closed registration refusals; a transport failure never authorizes fresh intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PortableIssuerRegistrationRefusal {
    Binding,
    Capacity,
    Clock,
    Conflict,
    Custody,
    Expired,
    Invalid,
    Possession,
    Retired,
    Storage,
    Transport,
    Unavailable,
}

/// Lookup reports retained work only; it never creates a challenge or approval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PortableIssuerRegistrationStatus {
    Preparing,
    Challenge(PortableIssuerChallenge),
    Registered(PortableIssuerEnrollment),
}

#[cfg(test)]
mod tests;

/// Root-authenticated request for a fresh issuer-local key preparation operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PreparePortableIssuerKey {
    pub operation_id: [u8; 32],
    pub custody: PortableIssuerCustody,
}

/// Public preparation result; this is not Root approval or issuance authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuerPreparedKey {
    pub operation_id: [u8; 32],
    pub custody: PortableIssuerCustody,
    pub public_key: [u8; 32],
}
