//! Cross-cutting performance instrumentation.
//!
//! This module provides instruction-count measurement primitives used
//! across interface, ops, timers, and background tasks.
//!
//! It is intentionally crate-level infrastructure, not part of the
//! domain layering (interface → ops → model).

use canic_cdk::candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell, collections::HashMap};

thread_local! {
    /// Last snapshot used by the `perf!` macro.
    pub static PERF_LAST: RefCell<u64> =
        RefCell::new(perf_counter());

    /// Aggregated perf counters keyed by label.
    static PERF_TABLE: RefCell<HashMap<String, PerfSlot>> = RefCell::new(HashMap::new());
}

// perf_counter
// until we need separate counter types, just stick to 1
#[must_use]
pub fn perf_counter() -> u64 {
    crate::cdk::api::performance_counter(0)
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
/// Aggregated perf counters keyed by label.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PerfEntry {
    pub label: String,
    pub count: u64,
    pub total_instructions: u64,
}

/// Record an instruction delta under the provided label.
pub fn record(label: Cow<'_, str>, delta: u64) {
    PERF_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        table
            .entry(label.into_owned())
            .or_default()
            .increment(delta);
    });
}

/// Snapshot all recorded perf counters, sorted by label.
#[must_use]
pub fn entries() -> Vec<PerfEntry> {
    PERF_TABLE.with(|table| {
        let table = table.borrow();

        let mut out: Vec<PerfEntry> = table
            .iter()
            .map(|(label, slot)| PerfEntry {
                label: label.clone(),
                count: slot.count,
                total_instructions: slot.total_instructions,
            })
            .collect();

        out.sort_by(|a, b| a.label.cmp(&b.label));
        out
    })
}

#[cfg(test)]
pub fn reset() {
    PERF_TABLE.with(|t| t.borrow_mut().clear());
    PERF_LAST.with(|last| *last.borrow_mut() = 0);
}
