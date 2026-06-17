//! Module: ops::replay::ttl
//!
//! Responsibility: validate replay TTL bounds.
//! Does not own: receipt storage, command authorization, or replay decisions.
//! Boundary: replay guards call this before reserving receipts.

///
/// ReplayTtlError
///
/// Validation error emitted by replay TTL checks.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayTtlError {
    InvalidTtl { ttl_ns: u64, max_ttl_ns: u64 },
}

/// validate_replay_ttl
///
/// Enforce root replay TTL bounds as a pure mechanical check.
pub const fn validate_replay_ttl(ttl_ns: u64, max_ttl_ns: u64) -> Result<(), ReplayTtlError> {
    if ttl_ns == 0 || ttl_ns > max_ttl_ns {
        return Err(ReplayTtlError::InvalidTtl { ttl_ns, max_ttl_ns });
    }

    Ok(())
}
