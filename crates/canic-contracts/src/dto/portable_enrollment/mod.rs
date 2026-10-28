//! Passive portable enrollment exchange. Decoding never installs trust.

use crate::ids::FleetKey;
use ic_platform::Principal;
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

#[cfg(test)]
mod tests;
