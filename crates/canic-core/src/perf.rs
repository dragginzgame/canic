//! Cross-cutting performance instrumentation.
//!
//! This module provides instruction-count measurement primitives used
//! across endpoints, ops, timers, and background tasks.
//!
//! It is intentionally crate-level infrastructure, not part of the
//! domain layering (endpoints → ops → model).

use canic_cdk::candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    /// Last snapshot used by the `perf!` macro.
    #[cfg(not(test))]
    pub static PERF_LAST: RefCell<u64> = RefCell::new(perf_counter());

    // Unit tests run outside a canister context, so `perf_counter()` would trap.
    #[cfg(test)]
    pub static PERF_LAST: RefCell<u64> = const { RefCell::new(0) };

    /// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
    static PERF_TABLE: RefCell<HashMap<PerfKey, PerfSlot>> = RefCell::new(HashMap::new());

    /// Stack of active endpoint scopes for exclusive instruction accounting.
    /// This is independent of `PERF_LAST`, which is only used by `perf!` checkpoints.
    static PERF_STACK: RefCell<Vec<PerfFrame>> = const { RefCell::new(Vec::new()) };
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
/// PerfFrame
/// Tracks an active endpoint scope and accumulated child instructions.
///

struct PerfFrame {
    start: u64,
    child_instructions: u64,
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

pub fn record_endpoint(func: &str, delta_instructions: u64) {
    record(PerfKey::Endpoint(func.to_string()), delta_instructions);
}

pub fn record_timer(label: &str, delta_instructions: u64) {
    record(PerfKey::Timer(label.to_string()), delta_instructions);
}

/// Begin an endpoint scope and push it on the stack.
pub(crate) fn enter_endpoint() {
    enter_endpoint_at(perf_counter());
}

/// End the most recent endpoint scope and record exclusive instructions.
pub(crate) fn exit_endpoint(label: &str) {
    exit_endpoint_at(label, perf_counter());
}

fn enter_endpoint_at(start: u64) {
    PERF_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();

        // If a previous call trapped, clear any stale frames.
        if let Some(last) = stack.last()
            && start < last.start
        {
            stack.clear();
        }

        stack.push(PerfFrame {
            start,
            child_instructions: 0,
        });
    });
}

fn exit_endpoint_at(label: &str, end: u64) {
    PERF_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        let Some(frame) = stack.pop() else {
            record_endpoint(label, end);
            return;
        };

        let total = end.saturating_sub(frame.start);
        let exclusive = total.saturating_sub(frame.child_instructions);

        if let Some(parent) = stack.last_mut() {
            parent.child_instructions = parent.child_instructions.saturating_add(total);
        }

        record_endpoint(label, exclusive);
    });
}

/// Snapshot all recorded perf counters, sorted by key.
/// Entries are sorted by (kind, label).
#[must_use]
pub fn entries() -> Vec<PerfEntry> {
    PERF_TABLE.with(|table| {
        let table = table.borrow();

        let mut out: Vec<PerfEntry> = table
            .iter()
            .map(|(key, slot)| PerfEntry {
                key: key.clone(),
                count: slot.count,
                total_instructions: slot.total_instructions,
            })
            .collect();

        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    })
}

///
/// TESTS
///

#[cfg(test)]
pub fn reset() {
    PERF_TABLE.with(|t| t.borrow_mut().clear());
    PERF_LAST.with(|last| *last.borrow_mut() = 0);
    PERF_STACK.with(|stack| stack.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint_at(now: u64) {
        PERF_LAST.with(|last| *last.borrow_mut() = now);
    }

    fn entry_for(label: &str) -> PerfEntry {
        entries()
            .into_iter()
            .find(|entry| matches!(&entry.key, PerfKey::Endpoint(l) if l == label))
            .expect("expected perf entry to exist")
    }

    #[test]
    fn nested_endpoints_record_exclusive_totals() {
        reset();

        enter_endpoint_at(100);
        checkpoint_at(140);

        enter_endpoint_at(200);
        checkpoint_at(230);
        exit_endpoint_at("child", 260);

        exit_endpoint_at("parent", 300);

        let parent = entry_for("parent");
        let child = entry_for("child");

        assert_eq!(child.count, 1);
        assert_eq!(child.total_instructions, 60);
        assert_eq!(parent.count, 1);
        assert_eq!(parent.total_instructions, 140);
    }
}
