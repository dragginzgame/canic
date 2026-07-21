///
/// CycleTracker retention policy.
///

pub const CYCLE_TRACKER_RETENTION_SECS: u64 = 60 * 60 * 24 * 7; // ~7 days
pub const CYCLE_TOPUP_MIN_CHECK_SECS: u64 = 60;
pub const CYCLE_TOPUP_MAX_CHECK_SECS: u64 = 60 * 60;
pub const CYCLE_TOPUP_OBSERVATION_MAX_AGE_SECS: u64 = 60 * 60 * 12;
pub const CYCLE_TOPUP_FUNDING_ALLOWANCE_SECS: u64 = 60 * 5;
pub const CYCLE_TOPUP_SAFETY_MARGIN_SECS: u64 = 60 * 5;

const SECONDS_PER_DAY: u128 = 60 * 60 * 24;

#[must_use]
pub const fn retention_cutoff(now: u64) -> u64 {
    now.saturating_sub(CYCLE_TRACKER_RETENTION_SECS)
}

/// One prior balance observation eligible for burn-rate estimation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CycleBalanceObservation {
    pub timestamp_secs: u64,
    pub balance: u128,
}

/// Pure scheduling result for one configured automatic-top-up owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CycleTopupTiming {
    Due,
    CheckAfter { delay_secs: u64 },
}

/// Derive the next funding-safety observation from balance headroom.
#[must_use]
pub fn cycle_topup_timing(
    now_secs: u64,
    current_balance: u128,
    threshold: u128,
    previous: Option<CycleBalanceObservation>,
) -> CycleTopupTiming {
    if current_balance <= threshold {
        return CycleTopupTiming::Due;
    }

    let burn_rate = conservative_burn_rate(now_secs, current_balance, threshold, previous);
    let headroom = current_balance.saturating_sub(threshold);
    let crossing_secs = headroom / burn_rate;
    let reserve_secs = u128::from(
        CYCLE_TOPUP_FUNDING_ALLOWANCE_SECS.saturating_add(CYCLE_TOPUP_SAFETY_MARGIN_SECS),
    );
    let unconstrained_delay = crossing_secs.saturating_sub(reserve_secs);
    let delay_secs = u64::try_from(unconstrained_delay)
        .unwrap_or(u64::MAX)
        .clamp(CYCLE_TOPUP_MIN_CHECK_SECS, CYCLE_TOPUP_MAX_CHECK_SECS);

    CycleTopupTiming::CheckAfter { delay_secs }
}

fn conservative_burn_rate(
    now_secs: u64,
    current_balance: u128,
    threshold: u128,
    previous: Option<CycleBalanceObservation>,
) -> u128 {
    let floor = ceil_div(threshold, SECONDS_PER_DAY).max(1);
    let Some(previous) = previous else {
        return floor;
    };
    let elapsed = now_secs.saturating_sub(previous.timestamp_secs);
    if elapsed == 0
        || elapsed > CYCLE_TOPUP_OBSERVATION_MAX_AGE_SECS
        || current_balance >= previous.balance
    {
        return floor;
    }

    let observed = ceil_div(
        previous.balance.saturating_sub(current_balance),
        u128::from(elapsed),
    );
    observed.saturating_mul(2).max(floor)
}

const fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    let quotient = numerator / denominator;
    if numerator.is_multiple_of(denominator) {
        quotient
    } else {
        quotient.saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TC: u128 = 1_000_000_000_000;

    #[test]
    fn safety_observation_window_is_one_hour() {
        assert_eq!(CYCLE_TOPUP_MAX_CHECK_SECS, 60 * 60);
    }

    #[test]
    fn topup_is_due_at_or_below_the_threshold() {
        assert_eq!(
            cycle_topup_timing(100, 10 * TC, 10 * TC, None),
            CycleTopupTiming::Due
        );
        assert_eq!(
            cycle_topup_timing(100, 10 * TC - 1, 10 * TC, None),
            CycleTopupTiming::Due
        );
    }

    #[test]
    fn insufficient_history_uses_the_threshold_burn_floor_and_bounds() {
        assert_eq!(
            cycle_topup_timing(100, 20 * TC, 10 * TC, None),
            CycleTopupTiming::CheckAfter {
                delay_secs: CYCLE_TOPUP_MAX_CHECK_SECS,
            }
        );
        assert_eq!(
            cycle_topup_timing(100, 10 * TC + 1, 10 * TC, None),
            CycleTopupTiming::CheckAfter {
                delay_secs: CYCLE_TOPUP_MIN_CHECK_SECS,
            }
        );
    }

    #[test]
    fn recent_observed_burn_is_doubled_and_can_advance_the_check() {
        let previous = CycleBalanceObservation {
            timestamp_secs: 100,
            balance: 20 * TC,
        };
        assert_eq!(
            cycle_topup_timing(200, 19 * TC, 10 * TC, Some(previous)),
            CycleTopupTiming::CheckAfter {
                delay_secs: CYCLE_TOPUP_MIN_CHECK_SECS,
            }
        );
    }

    #[test]
    fn stale_flat_or_deposit_observations_use_the_floor() {
        for previous in [
            CycleBalanceObservation {
                timestamp_secs: 100,
                balance: 20 * TC,
            },
            CycleBalanceObservation {
                timestamp_secs: 100,
                balance: 19 * TC,
            },
            CycleBalanceObservation {
                timestamp_secs: 100,
                balance: 18 * TC,
            },
        ] {
            assert_eq!(
                cycle_topup_timing(
                    100 + CYCLE_TOPUP_OBSERVATION_MAX_AGE_SECS + 1,
                    19 * TC,
                    10 * TC,
                    Some(previous),
                ),
                CycleTopupTiming::CheckAfter {
                    delay_secs: CYCLE_TOPUP_MAX_CHECK_SECS,
                }
            );
        }
    }
}
