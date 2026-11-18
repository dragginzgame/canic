//! Fresh IC entropy for application operations. Callers retain their own deadlines.
use ic_platform_protocol::raw_rand;
use ic_platform_runtime::call::{Call, Outcome, Wait};
use ic_platform_types::Principal;

/// The single source request failed, or its reply was invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The transport did not return a reply.
    Unavailable,
    /// Platform refused the management payload.
    InvalidFrame,
}

/// Request exactly 32 fresh bytes with an explicit bounded wait, in seconds.
/// # Errors
/// Propagates failure without issuing a replacement or producing fallback bytes.
pub async fn fresh(wait_seconds: u32) -> Result<[u8; 32], Error> {
    let call = Call::new(
        Principal::management_canister(),
        raw_rand::METHOD,
        Wait::Bounded {
            seconds: wait_seconds,
        },
    )
    .args(raw_rand::encode(6).map_err(|_| Error::InvalidFrame)?)
    .execute()
    .await
    .map_err(|_| Error::Unavailable)?;
    match call.into_parts().0 {
        Outcome::Reply(bytes) => raw_rand::decode(&bytes, 42).map_err(|_| Error::InvalidFrame),
        Outcome::Rejected(_) | Outcome::OutcomeUnknown(_) => Err(Error::Unavailable),
    }
}
