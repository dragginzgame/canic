//! Module: ops::auth::enrollment
//!
//! Responsibility: admit complete Root snapshots, monotonic metadata and retirements.
//! Boundary: authenticated registration owns approval; receiver pulls authenticate Root.

#[cfg(any(test, feature = "auth-root-delegation-state"))]
mod authority;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
mod receiver;
#[cfg(any(test, feature = "auth-root-delegation-state"))]
pub mod registration;
#[cfg(test)]
mod tests;
pub(super) mod validation;

#[cfg(any(test, feature = "auth-root-delegation-state"))]
use crate::model::auth::portable_registration::EnrollmentDomain;
#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer",
    feature = "auth-root-delegation-state"
))]
use canic_contracts::{ids::FleetKey, protocol::portable_enrollment as wire};
use ic_platform::Principal;
#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-root-delegation-state"
))]
use serde::Serialize;
use thiserror::Error;

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
    #[error("portable issuer custody profile is not qualified")]
    Custody,
    #[error("portable issuer registration intent was superseded or differs")]
    Conflict,
    #[error("portable issuer key possession is invalid")]
    Possession,
}

/// Protected runtime binding; never constructed from a browser request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer",
    feature = "auth-root-delegation-state"
))]
pub(super) struct EnrollmentBinding {
    pub authority: Principal,
    pub receiver: Principal,
    pub fleet: FleetKey,
}

impl From<PortableEnrollmentError>
    for canic_contracts::dto::portable_enrollment::PortableIssuerRegistrationRefusal
{
    fn from(error: PortableEnrollmentError) -> Self {
        use PortableEnrollmentError as E;
        match error {
            E::Unavailable => Self::Unavailable,
            E::Binding => Self::Binding,
            E::Invalid => Self::Invalid,
            E::Expired => Self::Expired,
            E::Clock => Self::Clock,
            E::Retired => Self::Retired,
            E::Capacity => Self::Capacity,
            E::Storage | E::Rollback | E::Generation => Self::Storage,
            E::Transport => Self::Transport,
            E::Custody => Self::Custody,
            E::Conflict => Self::Conflict,
            E::Possession => Self::Possession,
        }
    }
}

#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-root-delegation-state"
))]
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, PortableEnrollmentError> {
    ic_wire::native::encode(value, wire::RESPONSE_LIMITS)
        .map_err(|_| PortableEnrollmentError::Capacity)
}

#[cfg(any(test, feature = "auth-local-application-authorization"))]
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, PortableEnrollmentError> {
    ic_wire::native::decode(bytes, wire::RESPONSE_LIMITS)
        .map_err(|_| PortableEnrollmentError::Storage)
}

/// Pull one bounded snapshot directly from the selected protected Root.
#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer"
))]
pub(super) async fn fetch_snapshot(
    authority: Principal,
    challenge: [u8; 32],
) -> Result<
    canic_contracts::dto::portable_enrollment::PortableEnrollmentSnapshot,
    PortableEnrollmentError,
> {
    use ic_platform::call::{Call, WaitPolicy};
    let request = canic_contracts::dto::portable_enrollment::PortableEnrollmentRequest {
        boot_challenge: challenge,
    };
    let bytes = ic_wire::native::encode_request(wire::METHOD_ID, &request, wire::REQUEST_LIMITS)
        .map_err(|_| PortableEnrollmentError::Storage)?;
    let response = crate::ops::ic::call::execute(
        Call::new(
            authority,
            wire::METHOD,
            WaitPolicy::Bounded {
                timeout_seconds: 30,
            },
        )
        .take_raw_args(bytes),
    )
    .await
    .map_err(|_| PortableEnrollmentError::Transport)?;
    ic_wire::native::decode_response::<
        Result<
            canic_contracts::dto::portable_enrollment::PortableEnrollmentSnapshot,
            crate::dto::error::Error,
        >,
    >(
        wire::METHOD_ID,
        &response.into_bytes(),
        wire::RESPONSE_LIMITS,
    )
    .map_err(|_| PortableEnrollmentError::Storage)?
    .map_err(|_| PortableEnrollmentError::Unavailable)
}

/// Obtain fresh Auth randomness through the one canonical external management boundary.
#[cfg(any(
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer",
    feature = "auth-root-delegation-state"
))]
pub async fn fresh_challenge() -> Result<[u8; 32], PortableEnrollmentError> {
    use ic_platform::{
        call::{Call, WaitPolicy},
        candid::protocols::{ReplyArity, management::raw_rand},
    };
    let args = raw_rand::encode().map_err(|_| PortableEnrollmentError::Transport)?;
    let response = crate::ops::ic::call::execute(
        Call::new(
            Principal::management_canister(),
            raw_rand::METHOD,
            WaitPolicy::Bounded {
                timeout_seconds: 30,
            },
        )
        .take_raw_args(args),
    )
    .await
    .map_err(|_| PortableEnrollmentError::Transport)?;
    let limits = ic_platform::candid::DecodeLimits {
        reply_bytes: 128,
        decoding_quota: 4096,
        skipping_quota: 0,
        type_entries: 8,
        header_bytes: 64,
    };
    let bytes = raw_rand::decode(&response.into_bytes(), limits, ReplyArity::Exact)
        .map_err(|_| PortableEnrollmentError::Transport)?;
    let challenge: [u8; 32] = bytes
        .try_into()
        .map_err(|_| PortableEnrollmentError::Transport)?;
    if challenge == [0; 32] {
        return Err(PortableEnrollmentError::Invalid);
    }
    Ok(challenge)
}
