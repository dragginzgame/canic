//! Module: ops::runtime::metrics::inter_canister_call
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the inter_canister_call family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::ops::{prelude::*, runtime::metrics::system::SystemMetrics};
use std::{cell::RefCell, collections::HashMap};

const MAX_CALL_IDENTITIES: usize = 1_024;
const MAX_METHOD_BYTES: usize = 128;

/// Bounded target/method counters and calls whose identities were not retained.
#[derive(Default)]
struct InterCanisterCallTable {
    entries: HashMap<InterCanisterCallMetricKey, u64>,
    overflow: u64,
}

thread_local! {
    /// Thread-local storage for inter-canister call counters.
    ///
    /// Keyed by `(target, method)` and holding the number of calls observed.
    static INTER_CANISTER_CALL_METRICS: RefCell<InterCanisterCallTable> =
        RefCell::new(InterCanisterCallTable::default());
}

///
/// InterCanisterCallMetricsSnapshot
///
/// Point-in-time inter-canister call metric rows from the runtime counter table.
///

#[derive(Clone)]
pub struct InterCanisterCallMetricsSnapshot {
    pub entries: Vec<(InterCanisterCallMetricKey, u64)>,
    pub overflow: u64,
}

///
/// InterCanisterCallMetricKey
///
/// A retained identity has a method of at most 128 UTF-8 bytes.
///

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct InterCanisterCallMetricKey {
    pub target: Principal,
    pub method: String,
}

///
/// InterCanisterCallMetrics
///
/// Volatile counters for inter-canister calls keyed by target + method.
/// Retains the first 1,024 identities; excess or oversized identities count as overflow.
///

pub struct InterCanisterCallMetrics;

impl InterCanisterCallMetrics {
    /// Increment the inter-canister call counter for a target/method pair.
    fn increment(target: Principal, method: &str) {
        INTER_CANISTER_CALL_METRICS.with_borrow_mut(|table| {
            if method.len() > MAX_METHOD_BYTES {
                table.overflow = table.overflow.saturating_add(1);
                return;
            }

            let key = InterCanisterCallMetricKey {
                target,
                method: method.to_string(),
            };
            if let Some(count) = table.entries.get_mut(&key) {
                *count = count.saturating_add(1);
            } else if table.entries.len() < MAX_CALL_IDENTITIES {
                table.entries.insert(key, 1);
            } else {
                table.overflow = table.overflow.saturating_add(1);
            }
        });
    }

    /// Record call construction in system + ICC metrics, including unexecuted builders.
    ///
    /// This is the preferred integration point for call instrumentation,
    /// even if not all call paths currently route through it.
    pub fn record_call(target: impl Into<Principal>, method: &str) {
        let target: Principal = target.into();

        SystemMetrics::increment();
        Self::increment(target, method);
    }

    /// Snapshot the current inter-canister call metrics as a stable vector.
    #[must_use]
    pub fn snapshot() -> InterCanisterCallMetricsSnapshot {
        INTER_CANISTER_CALL_METRICS.with_borrow(|table| InterCanisterCallMetricsSnapshot {
            entries: table
                .entries
                .iter()
                .map(|(key, count)| (key.clone(), *count))
                .collect(),
            overflow: table.overflow,
        })
    }

    /// Test-only helper: clear all inter-canister call metrics.
    #[cfg(test)]
    pub fn reset() {
        INTER_CANISTER_CALL_METRICS.with_borrow_mut(|table| {
            *table = InterCanisterCallTable::default();
        });
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::runtime::metrics::system::SystemMetrics;

    fn snapshot_map() -> HashMap<InterCanisterCallMetricKey, u64> {
        InterCanisterCallMetrics::snapshot()
            .entries
            .into_iter()
            .collect()
    }

    #[test]
    fn inter_canister_call_metrics_track_target_and_method() {
        InterCanisterCallMetrics::reset();

        let t1 = Principal::from_slice(&[1; 29]);
        let t2 = Principal::from_slice(&[2; 29]);

        InterCanisterCallMetrics::increment(t1, "foo");
        InterCanisterCallMetrics::increment(t1, "foo");
        InterCanisterCallMetrics::increment(t1, "bar");
        InterCanisterCallMetrics::increment(t2, "foo");

        let map = snapshot_map();

        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target: t1,
                method: "foo".to_string()
            }),
            Some(&2)
        );

        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target: t1,
                method: "bar".to_string()
            }),
            Some(&1)
        );

        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target: t2,
                method: "foo".to_string()
            }),
            Some(&1)
        );

        assert_eq!(map.len(), 3);
    }

    #[test]
    fn record_call_updates_inter_canister_call_and_system_metrics() {
        InterCanisterCallMetrics::reset();
        SystemMetrics::reset();

        let target = Principal::from_slice(&[3; 29]);
        InterCanisterCallMetrics::record_call(target, "canic_sync");
        InterCanisterCallMetrics::record_call(target, "canic_sync");

        let map = snapshot_map();
        assert_eq!(
            map.get(&InterCanisterCallMetricKey {
                target,
                method: "canic_sync".to_string()
            }),
            Some(&2)
        );

        assert_eq!(SystemMetrics::count(), 2);
    }

    #[test]
    fn identities_are_bounded_while_retained_rows_keep_counting() {
        InterCanisterCallMetrics::reset();
        SystemMetrics::reset();
        let target = Principal::from_slice(&[4; 29]);
        for index in 0..MAX_CALL_IDENTITIES {
            InterCanisterCallMetrics::record_call(target, &format!("method_{index}"));
        }
        InterCanisterCallMetrics::record_call(target, "excess");
        InterCanisterCallMetrics::record_call(target, "excess");
        InterCanisterCallMetrics::record_call(target, "method_0");
        let snapshot = InterCanisterCallMetrics::snapshot();
        assert_eq!(snapshot.entries.len(), MAX_CALL_IDENTITIES);
        assert_eq!(snapshot.overflow, 2);
        assert_eq!(
            snapshot_map().get(&InterCanisterCallMetricKey {
                target,
                method: "method_0".to_string(),
            }),
            Some(&2)
        );
        assert_eq!(
            snapshot.entries.iter().map(|(_, count)| count).sum::<u64>() + snapshot.overflow,
            SystemMetrics::count()
        );
    }

    #[test]
    fn method_byte_bound_and_overflow_identity_are_unambiguous() {
        InterCanisterCallMetrics::reset();
        let target = Principal::anonymous();
        let maximum = "é".repeat(MAX_METHOD_BYTES / 2);
        InterCanisterCallMetrics::increment(target, &maximum);
        InterCanisterCallMetrics::increment(target, &format!("{maximum}x"));
        InterCanisterCallMetrics::increment(target, &"x".repeat(1_000_000));
        InterCanisterCallMetrics::increment(target, "overflow");
        let snapshot = InterCanisterCallMetrics::snapshot();
        assert_eq!(snapshot.entries.len(), 2);
        assert!(
            snapshot
                .entries
                .iter()
                .all(|(key, _)| key.method.len() <= MAX_METHOD_BYTES)
        );
        assert_eq!(snapshot.overflow, 2);
        InterCanisterCallMetrics::reset();
        let cleared = InterCanisterCallMetrics::snapshot();
        assert!(cleared.entries.is_empty());
        assert_eq!(cleared.overflow, 0);
    }

    #[test]
    fn retained_and_overflow_counts_saturate() {
        InterCanisterCallMetrics::reset();
        let target = Principal::anonymous();
        InterCanisterCallMetrics::increment(target, "retained");
        INTER_CANISTER_CALL_METRICS.with_borrow_mut(|table| {
            *table.entries.values_mut().next().unwrap() = u64::MAX;
            table.overflow = u64::MAX;
        });
        InterCanisterCallMetrics::increment(target, "retained");
        InterCanisterCallMetrics::increment(target, &"x".repeat(MAX_METHOD_BYTES + 1));
        let snapshot = InterCanisterCallMetrics::snapshot();
        assert_eq!(snapshot.entries[0].1, u64::MAX);
        assert_eq!(snapshot.overflow, u64::MAX);
    }

    #[test]
    fn target_churn_uses_the_same_cardinality_bound() {
        InterCanisterCallMetrics::reset();
        for index in 0..(MAX_CALL_IDENTITIES + 10) {
            let target = Principal::from_slice(&index.to_le_bytes());
            InterCanisterCallMetrics::increment(target, "method");
        }
        let snapshot = InterCanisterCallMetrics::snapshot();
        assert_eq!(snapshot.entries.len(), MAX_CALL_IDENTITIES);
        assert_eq!(snapshot.overflow, 10);
    }

    #[test]
    fn unexecuted_public_builders_record_bounded_construction_metrics() {
        InterCanisterCallMetrics::reset();
        SystemMetrics::reset();
        let target = Principal::anonymous();
        drop(crate::api::call::Call::bounded_wait(target, "method"));
        drop(crate::api::call::Call::unbounded_wait(
            target,
            &"x".repeat(MAX_METHOD_BYTES + 1),
        ));
        let snapshot = InterCanisterCallMetrics::snapshot();
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].1, 1);
        assert_eq!(snapshot.overflow, 1);
        assert_eq!(SystemMetrics::count(), 2);
    }
}
