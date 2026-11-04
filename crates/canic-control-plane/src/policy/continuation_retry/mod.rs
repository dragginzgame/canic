//! Bounded automatic business attempts; Timers retains callback execution ownership.

use canic_contracts::diagnostics::codes;

/// An operator-authorized attempt window permits at most this many attempts without progress.
pub const MAX_ATTEMPTS_WITHOUT_PROGRESS: u32 = 16;

/// Proved invalid authority or data needs review; dependency failures remain reconcilable.
pub fn requires_review(code: u16) -> bool {
    [
        codes::STATE_CONFLICT,
        codes::CODEC_CONFLICT,
        codes::AUTHORITY_CONFLICT,
        codes::AUTHORITY_UNAUTHORIZED,
        codes::STATE_INVALID,
    ]
    .into_iter()
    .any(|registered| registered.raw_code().raw() == code)
}

/// Bound observation and transport retries even when no diagnostic was produced.
pub fn delay_ns(attempts: u32) -> u64 {
    (1_u64 << attempts.saturating_sub(1).min(6)).min(60) * 1_000_000_000
}
