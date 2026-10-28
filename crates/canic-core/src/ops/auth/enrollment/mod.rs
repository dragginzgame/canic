//! Module: ops::auth::enrollment
//!
//! Responsibility: admit complete Root snapshots, monotonic metadata and retirements.
//! Boundary: authenticated registration owns approval; receiver pulls authenticate Root.

#[cfg(any(test, feature = "auth-root-delegation-state"))]
mod authority;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
mod receiver;
#[cfg(test)]
mod tests;
mod validation;

use canic_contracts::{
    dto::portable_enrollment::*, ids::FleetKey, protocol::portable_enrollment as wire,
};
use ic_platform::Principal;
#[cfg(any(test, feature = "auth-root-delegation-state"))]
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use validation::{key_id, validate_contents};

#[cfg(any(test, feature = "auth-root-delegation-state"))]
pub use authority::RootPortableEnrollmentOps;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub use receiver::invalidate_receiver_boot;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub use receiver::{
    AuthenticatedPortableEnrollment, PortableEnrollmentPull, ReceiverPortableEnrollment,
};

/// Closed enrollment failures, never application authorization.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PortableEnrollmentError {
    #[error("portable enrollment is unavailable")]
    Unavailable,
    #[error("portable enrollment authority or pull binding differs")]
    Binding,
    #[error("portable enrollment shape or grants are invalid")]
    Invalid,
    #[error("portable enrollment is expired or the receipt is late")]
    Expired,
    #[error("portable enrollment clock moved backwards")]
    Clock,
    #[error("portable enrollment revision or generation rolled back")]
    Rollback,
    #[error("portable enrollment changed without advancing authority generation")]
    Generation,
    #[error("portable enrollment would revive a retired key")]
    Retired,
    #[error("portable enrollment capacity or counter is exhausted")]
    Capacity,
    #[error("portable enrollment encoding or storage is invalid")]
    Storage,
    #[error("portable enrollment Root transport failed")]
    Transport,
}

/// Protected runtime binding; never constructed from a browser request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EnrollmentBinding {
    authority: Principal,
    receiver: Principal,
    fleet: FleetKey,
}

#[cfg(any(test, feature = "auth-root-delegation-state"))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct EnrollmentDomain {
    authority: Principal,
    fleet: FleetKey,
}

#[cfg(any(test, feature = "auth-root-delegation-state"))]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct RegistryContents {
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    domain: Option<EnrollmentDomain>,
    revision: u64,
    generation: u64,
    entries: Vec<PortableIssuerEnrollment>,
    retired_keys: Vec<[u8; 32]>,
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, PortableEnrollmentError> {
    ic_wire::native::encode(value, wire::RESPONSE_LIMITS)
        .map_err(|_| PortableEnrollmentError::Capacity)
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, PortableEnrollmentError> {
    ic_wire::native::decode(bytes, wire::RESPONSE_LIMITS)
        .map_err(|_| PortableEnrollmentError::Storage)
}
