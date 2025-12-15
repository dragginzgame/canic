//! Cross-cutting performance instrumentation.
//!
//! This module provides instruction-count measurement primitives used
//! across endpoints, ops, timers, and background tasks.
//!
//! It is intentionally crate-level infrastructure, not part of the
//! domain layering (endpoints → ops → model).

use canic_cdk::candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell, collections::HashMap};

thread_local! {
    /// Last snapshot used by the `perf!` macro.
    #[cfg(not(test))]
    pub static PERF_LAST: RefCell<u64> = RefCell::new(perf_counter());

    // Unit tests run outside a canister context, so `perf_counter()` would trap.
    #[cfg(test)]
    pub static PERF_LAST: RefCell<u64> = const { RefCell::new(0) };

    /// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
    static PERF_TABLE: RefCell<HashMap<PerfKey, PerfSlot>> = RefCell::new(HashMap::new());
}

/// Returns the **call-context instruction counter** for the current execution.
///
/// This value is obtained from `ic0.performance_counter(1)` and represents the
/// total number of WebAssembly instructions executed by *this canister* within
/// the **current call context**.
///
/// Key properties:
/// - Monotonically increasing for the duration of the call context
/// - Accumulates across `await` points and resumptions
/// - Resets only when a new call context begins
/// - Counts *only* instructions executed by this canister (not other canisters)
///
/// This counter is suitable for:
/// - Endpoint-level performance accounting
/// - Async workflows and timers
/// - Regression detection and coarse-grained profiling
///
/// It is **not** a measure of cycle cost. Expensive inter-canister operations
/// (e.g., canister creation) may have low instruction counts here but high cycle
/// charges elsewhere.
///
/// For fine-grained, single-slice profiling (e.g., hot loops), use
/// `ic0.performance_counter(0)` instead.
#[must_use]
pub fn perf_counter() -> u64 {
    crate::cdk::api::performance_counter(1)
}

///
/// PerfKey
/// splitting up by Timer type to avoid confusing string comparisons
///

#[derive(
    CandidType, Clone, Debug, Deserialize, Serialize, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
pub enum PerfKey {
    Endpoint(String),
    Timer(String),
}

///
/// PerfSlot
///

#[derive(Default)]
struct PerfSlot {
    count: u64,
    total_instructions: u64,
}

impl PerfSlot {
    const fn increment(&mut self, delta: u64) {
        self.count = self.count.saturating_add(1);
        self.total_instructions = self.total_instructions.saturating_add(delta);
    }
}

///
/// PerfEntry
/// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PerfEntry {
    pub label: String,
    pub key: PerfKey,
    pub count: u64,
    pub total_instructions: u64,
}

/// Record a counter under the provided key.
pub fn record(key: PerfKey, delta: u64) {
    PERF_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        table.entry(key).or_default().increment(delta);
    });
}

pub fn record_endpoint(func: Cow<'_, str>, total_instructions: u64) {
    record(PerfKey::Endpoint(func.into_owned()), total_instructions);
}

pub fn record_timer(label: Cow<'_, str>, delta_instructions: u64) {
    record(PerfKey::Timer(label.into_owned()), delta_instructions);
}

/// Snapshot all recorded perf counters, sorted by key.
#[must_use]
pub fn entries() -> Vec<PerfEntry> {
    PERF_TABLE.with(|table| {
        let table = table.borrow();

        let mut out: Vec<PerfEntry> = table
            .iter()
            .map(|(key, slot)| PerfEntry {
                label: match key {
                    PerfKey::Endpoint(label) | PerfKey::Timer(label) => label.clone(),
                },
                key: key.clone(),
                count: slot.count,
                total_instructions: slot.total_instructions,
            })
            .collect();

        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    })
}

#[cfg(test)]
pub fn reset() {
    PERF_TABLE.with(|t| t.borrow_mut().clear());
    PERF_LAST.with(|last| *last.borrow_mut() = 0);
}
