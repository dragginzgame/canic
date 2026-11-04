//! Issuer requests and original results; Auth Wire owns the credential envelope.

#[cfg(test)]
mod tests;

use crate::dto::portable_enrollment::PortableEnrollmentGrant;
use serde::{Deserialize, Serialize};

/// Account caller selects its operation and browser login, never its subject or audience.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IssuePortableCredential {
    pub epoch: u64,
    pub operation_id: [u8; 32],
    pub login_epoch: u64,
    pub login_id: [u8; 32],
    pub browser_public_key: [u8; 32],
    pub grants: Vec<PortableEnrollmentGrant>,
}

/// Lookup selects only the actual account caller's original operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReconcilePortableCredential {
    pub epoch: u64,
    pub operation_id: [u8; 32],
}

/// Current operation epoch, read without creating work or reserving a login.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuanceContext {
    pub epoch: u64,
}

/// Bounded reclamation result; advancing the epoch precedes forgetting any history.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PortableIssuanceMaintenance {
    pub epoch: u64,
    pub removed_issuances: u32,
    pub removed_logins: u32,
}

/// Original Auth credential and absolute admission deadline, never renewed at retrieval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IssuedPortableCredential {
    pub credential: ic_auth_wire::model::PortableCredential,
    pub admission_deadline_ns: u64,
}

/// Missing or unfinished work is unresolved, not evidence that issuance had no effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PortableCredentialIssuanceStatus {
    Issued(IssuedPortableCredential),
    Unresolved,
}

/// Typed issuer failures; no refusal authorizes a replacement operation automatically.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PortableCredentialIssuanceRefusal {
    Binding,
    Capacity,
    Clock,
    Conflict,
    Denied,
    EpochClosed,
    Expired,
    Invalid,
    Storage,
    Transport,
    Unavailable,
}
