//! Module: policy::funding_acceptance
//!
//! Responsibility: classify explicit availability refusals.
//! Does not own: transport, persistence, funding effects or timer scheduling.
//! Boundary: invalid authority and unknown error codes remain terminal.

use canic_contracts::diagnostics::codes;

/// Only explicit unavailable conditions permit future automatic demand observations.
pub fn is_temporary(code: u16) -> bool {
    [
        codes::STATE_UNAVAILABLE,
        codes::AUTHORITY_UNAVAILABLE,
        codes::CAPACITY_UNAVAILABLE,
    ]
    .into_iter()
    .any(|known| known.raw_code().raw() == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_is_distinct_from_invalid_authority_and_unknown_errors() {
        for code in [
            codes::STATE_UNAVAILABLE,
            codes::AUTHORITY_UNAVAILABLE,
            codes::CAPACITY_UNAVAILABLE,
        ] {
            assert!(is_temporary(code.raw_code().raw()));
        }
        for code in [
            codes::STATE_CONFLICT,
            codes::STATE_INVALID,
            codes::AUTHORITY_UNAUTHORIZED,
            codes::AUTHORITY_CONFLICT,
            codes::REQUEST_INVALID,
        ] {
            assert!(!is_temporary(code.raw_code().raw()));
        }
        assert!(!is_temporary(u16::MAX));
    }
}
