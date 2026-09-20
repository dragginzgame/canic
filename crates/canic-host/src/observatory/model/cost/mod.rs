//! Module: observatory::model::cost
//!
//! Responsibility: numeric source observations and interval arithmetic inputs.
//! Boundary: no wire parsing, collection or persisted accounting authority.

///
/// CostInterval
///
/// Source-clock bounds used by host comparison policy.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::observatory) struct CostInterval {
    pub start_ns: u64,
    pub end_ns: u64,
}

///
/// CostCounterReading
///
/// Parsed cumulative observation within an independently checked source heap.
///

#[derive(Clone, Copy)]
pub(in crate::observatory) struct CostCounterReading {
    pub value: u128,
    pub observed_at_ns: u64,
    pub window_id: u64,
    pub saturated: bool,
}

///
/// SignedCycleAmount
///
/// Exact signed magnitude for host arithmetic across the full u128 cycle range.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::observatory) struct SignedCycleAmount {
    pub negative: bool,
    pub magnitude: u128,
}

///
/// CounterMovement
///
/// Host policy's cumulative-counter movement over an exact source interval.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::observatory) struct CounterMovement {
    pub interval: CostInterval,
    pub amount: u128,
}

///
/// CostIntervalIssue
///
/// Pure numeric rejection reasons, converted into host report failures by ops.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::observatory) enum CostIntervalIssue {
    CounterDecreased,

    CounterWindowChanged,

    NonAdvancingWindow,

    Overflow,

    SaturatedCounter,
}
