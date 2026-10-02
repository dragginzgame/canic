//! Pure whole-operation capacity-import budget arithmetic.
//!
//! Runtime owns call-cost quotes; this policy owns bounded call counts and overflow rejection.

// Each advance takes a fresh subnet and status sample. Confirmations continue
// into their next mutation without another observation-only advance.
const STOP_CALLS: u32 = 2 + 1;
const CONTROLLERS_CALLS: u32 = 2 + 1;
const UNINSTALL_CALLS: u32 = 2 + 1 + 1;
const READY_CALLS: u32 = 2 + 1;
const SOURCE_CALLS: u32 = STOP_CALLS + CONTROLLERS_CALLS + UNINSTALL_CALLS + READY_CALLS;
const TERMINAL_CALLS: u32 = 1;
const TERMINAL_RETRY_CALLS: u32 = 16;

/// Four advances use two observations each, three mutations and two history reads.
/// The terminal Root observation is shared by the complete operation.
#[must_use]
pub fn minimum_calls(sources: usize) -> Option<u32> {
    if sources == 0 {
        return None;
    }
    u32::try_from(sources)
        .ok()?
        .checked_mul(SOURCE_CALLS)?
        .checked_add(TERMINAL_CALLS)
}

/// Include bounded reconciliation retries and terminal observation retries.
#[must_use]
pub fn recommended_calls(sources: usize) -> Option<u32> {
    minimum_calls(sources)?;
    u32::try_from(sources)
        .ok()?
        .checked_mul(SOURCE_CALLS.checked_mul(2)?)?
        .checked_add(TERMINAL_RETRY_CALLS)
}

/// Reserve enough for every permitted call at the destination's largest current quote.
#[must_use]
pub fn required_debit(maximum_call_cycles: u128, calls: u32) -> Option<u128> {
    if maximum_call_cycles == 0 || calls == 0 {
        return None;
    }
    maximum_call_cycles.checked_mul(u128::from(calls))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_mainnet_path_and_bounded_retries_fit_the_reviewed_envelope() {
        let sources = 24;
        let calls = recommended_calls(sources).unwrap();
        let quote = 50_000_000_000;
        assert!(calls >= minimum_calls(sources).unwrap());
        let budget = required_debit(quote, calls).unwrap();
        let mut retained = budget;
        for _ in 0..calls {
            retained = retained.checked_sub(quote).unwrap();
        }
        assert_eq!(retained, 0);
        assert!(
            4_000_000_000_000 < required_debit(quote, minimum_calls(sources).unwrap()).unwrap()
        );
    }

    #[test]
    fn empty_or_overflowing_authority_has_no_budget() {
        assert_eq!(minimum_calls(0), None);
        assert_eq!(recommended_calls(usize::MAX), None);
        assert_eq!(required_debit(0, 1), None);
        assert_eq!(required_debit(1, 0), None);
        assert_eq!(required_debit(u128::MAX, 2), None);
    }

    #[test]
    fn eight_running_sources_reject_the_exhausted_reported_envelope() {
        let minimum = minimum_calls(8).unwrap();
        assert_eq!(minimum, 105);
        assert!(72 < minimum);
        let recommended = recommended_calls(8).unwrap();
        assert_eq!(recommended, 224);
        assert!(recommended > minimum.checked_mul(2).unwrap());
    }
}
