//! Fresh IC entropy for application operations. Callers retain their own deadlines.
use ic_platform_runtime::raw_rand;

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
    raw_rand::request(wait_seconds)
        .await
        .map_err(|error| match error {
            raw_rand::Error::Frame(_) => Error::InvalidFrame,
            raw_rand::Error::Submission(_)
            | raw_rand::Error::Rejected(_)
            | raw_rand::Error::OutcomeUnknown(_) => Error::Unavailable,
        })
}
