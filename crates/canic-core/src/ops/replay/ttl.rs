/// ReplayTtlError
///
/// Validation error emitted by replay TTL checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayTtlError {
    InvalidTtl {
        ttl_seconds: u64,
        max_ttl_seconds: u64,
    },
}

/// validate_replay_ttl
///
/// Enforce root replay TTL bounds as a pure mechanical check.
pub const fn validate_replay_ttl(
    ttl_seconds: u64,
    max_ttl_seconds: u64,
) -> Result<(), ReplayTtlError> {
    if ttl_seconds == 0 || ttl_seconds > max_ttl_seconds {
        return Err(ReplayTtlError::InvalidTtl {
            ttl_seconds,
            max_ttl_seconds,
        });
    }

    Ok(())
}
