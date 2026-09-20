//! Module: observatory::policy::cost
//!
//! Responsibility: exact cycle differences and counter interval admission.
//! Boundary: consumes parsed numeric observations; no I/O, wire types or price assumptions.

#[cfg(test)]
mod tests;

use crate::observatory::model::cost::{
    CostCounterReading, CostInterval, CostIntervalIssue, CounterMovement, SignedCycleAmount,
};

/// Admit only a strictly advancing source-clock interval.
pub(in crate::observatory) const fn interval(
    start_ns: u64,
    end_ns: u64,
) -> Result<CostInterval, CostIntervalIssue> {
    if end_ns <= start_ns {
        return Err(CostIntervalIssue::NonAdvancingWindow);
    }
    Ok(CostInterval { start_ns, end_ns })
}

/// Reject saturation, reset windows and decreases before projecting a counter delta.
pub(in crate::observatory) fn counter(
    before: CostCounterReading,
    after: CostCounterReading,
) -> Result<CounterMovement, CostIntervalIssue> {
    if before.saturated || after.saturated {
        return Err(CostIntervalIssue::SaturatedCounter);
    }
    if before.window_id != after.window_id {
        return Err(CostIntervalIssue::CounterWindowChanged);
    }
    let interval = interval(before.observed_at_ns, after.observed_at_ns)?;
    let amount = after
        .value
        .checked_sub(before.value)
        .ok_or(CostIntervalIssue::CounterDecreased)?;
    Ok(CounterMovement { interval, amount })
}

pub(in crate::observatory) const fn difference(left: u128, right: u128) -> SignedCycleAmount {
    SignedCycleAmount {
        negative: left < right,
        magnitude: left.abs_diff(right),
    }
}

/// Reconcile observed grants only: opening + incoming - outgoing - closing.
/// Cancel opposing terms before addition so representable results never overflow transiently.
pub(in crate::observatory) fn grant_adjusted_decrease(
    opening: u128,
    closing: u128,
    incoming: u128,
    outgoing: u128,
) -> Result<SignedCycleAmount, CostIntervalIssue> {
    let balance = difference(opening, closing);
    let grants = difference(incoming, outgoing);
    if balance.negative == grants.negative {
        Ok(SignedCycleAmount {
            negative: balance.negative,
            magnitude: balance
                .magnitude
                .checked_add(grants.magnitude)
                .ok_or(CostIntervalIssue::Overflow)?,
        })
    } else if balance.magnitude >= grants.magnitude {
        let magnitude = balance.magnitude - grants.magnitude;
        Ok(SignedCycleAmount {
            negative: magnitude != 0 && balance.negative,
            magnitude,
        })
    } else {
        Ok(SignedCycleAmount {
            negative: grants.negative,
            magnitude: grants.magnitude - balance.magnitude,
        })
    }
}
