use crate::cdk::api::performance_counter;
use canic_cdk::candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    pub static PERF_LAST: RefCell<u64> = RefCell::new(performance_counter(1));
    static PERF_TABLE: RefCell<HashMap<String, PerfSlot>> = RefCell::new(HashMap::new());
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

// wrapper around performance_counter just in case
#[must_use]
#[allow(clippy::missing_const_for_fn)]
pub fn perf_counter() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        performance_counter(1)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

/// Record an instruction delta under the provided label.
pub fn record(label: &str, delta: u64) {
    PERF_TABLE.with_borrow_mut(|table| {
        table.entry(label.to_string()).or_default().increment(delta);
    });
}

/// Snapshot all recorded perf counters, sorted by label.
#[must_use]
pub fn entries() -> Vec<PerfEntry> {
    PERF_TABLE.with_borrow(|table| {
        let mut entries: Vec<PerfEntry> = table
            .iter()
            .map(|(label, slot)| PerfEntry {
                label: label.clone(),
                count: slot.count,
                total_instructions: slot.total_instructions,
            })
            .collect();

        entries.sort_by(|a, b| a.label.cmp(&b.label));
        entries
    })
}

#[cfg(test)]
pub fn reset() {
    PERF_TABLE.with_borrow_mut(HashMap::clear);
    PERF_LAST.with_borrow_mut(|last| *last = 0);
}
