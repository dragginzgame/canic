//! Module: model::public_metrics
//!
//! Responsibility: retain bounded ephemeral aggregate snapshots.
//! Does not own: collection, endpoint authorization, or wire conversion.
//! Boundary: caches may be rebuilt after restoration and never carry authority.

mod history;

use crate::{
    cdk::types::Principal,
    domain::public_metrics::{PublicMetricFamily, PublicMetricKind},
};

use std::{cell::RefCell, collections::BTreeMap};

pub use history::{
    MAX_HISTORY_BYTES, MAX_HISTORY_SERIES, PUBLIC_HISTORY_SLOTS, PublicHistoryCache,
    PublicHistorySample,
};

/// Maximum rows retained per family, independent of caller pagination.
pub const MAX_PUBLIC_METRICS: usize = 256;
/// Maximum bytes in a public metric name or unit.
pub const MAX_PUBLIC_METRIC_TEXT_BYTES: usize = 128;
/// Sampling skips missed five-minute slots instead of catching up.
pub const PUBLIC_METRICS_CADENCE_NS: u64 = 300_000_000_000;
/// Permit one minute of timer jitter beyond the sampling cadence.
pub const PUBLIC_METRICS_STALE_AFTER_NS: u64 = 360_000_000_000;
/// History covers the current slot and the preceding 287 slots.
pub const PUBLIC_HISTORY_RETENTION_NS: u64 = 86_400_000_000_000;

/// One retained aggregate sample with no user or partition-key dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicMetricSample {
    pub name: String,
    pub canister_id: Option<Principal>,
    pub value: u128,
    pub unit: String,
    pub observed_at_ns: u64,
    pub kind: PublicMetricKind,
}

/// Ephemeral snapshot of one bounded metric family.
#[derive(Clone, Debug)]
pub struct PublicMetricSnapshot {
    pub sampled_at_ns: u64,
    pub truncated: bool,
    pub metrics: Vec<PublicMetricSample>,
}

thread_local! {
    static SNAPSHOTS: RefCell<BTreeMap<PublicMetricFamily, PublicMetricSnapshot>> = RefCell::default();
}

/// Retain validated aggregate snapshots and return immutable copies.
pub struct PublicMetricsCache;

impl PublicMetricsCache {
    pub fn replace(
        family: PublicMetricFamily,
        sampled_at_ns: u64,
        metrics: impl IntoIterator<Item = PublicMetricSample>,
    ) -> Result<(), crate::InternalError> {
        let mut metrics: Vec<_> = metrics.into_iter().take(MAX_PUBLIC_METRICS + 1).collect();
        let truncated = metrics.len() > MAX_PUBLIC_METRICS;
        metrics.truncate(MAX_PUBLIC_METRICS);
        if metrics.iter().any(|row| {
            row.name.is_empty()
                || row.unit.is_empty()
                || row.name.len() > MAX_PUBLIC_METRIC_TEXT_BYTES
                || row.unit.len() > MAX_PUBLIC_METRIC_TEXT_BYTES
                || row.observed_at_ns > sampled_at_ns
        }) {
            return Err(crate::InternalError::invalid_input());
        }
        for row in &mut metrics {
            if row.name.capacity() > MAX_PUBLIC_METRIC_TEXT_BYTES {
                row.name = row.name.as_str().into();
            }
            if row.unit.capacity() > MAX_PUBLIC_METRIC_TEXT_BYTES {
                row.unit = row.unit.as_str().into();
            }
        }
        metrics.sort_by(sample_order);
        if metrics
            .windows(2)
            .any(|pair| pair[0].name == pair[1].name && pair[0].canister_id == pair[1].canister_id)
        {
            return Err(crate::InternalError::invalid_input());
        }
        let invalid_source = SNAPSHOTS.with_borrow(|cache| {
            cache.get(&family).is_some_and(|prior| {
                let mut previous = prior.metrics.iter().peekable();
                metrics.iter().any(|row| {
                    while previous
                        .peek()
                        .is_some_and(|old| sample_order(old, row).is_lt())
                    {
                        previous.next();
                    }
                    previous.peek().is_some_and(|old| {
                        sample_order(old, row).is_eq() && row.observed_at_ns < old.observed_at_ns
                    })
                })
            })
        });
        if invalid_source {
            return Err(crate::InternalError::invalid_input());
        }
        PublicHistoryCache::record(family, sampled_at_ns, &metrics);
        let sampled_at_ns = metrics
            .iter()
            .map(|row| row.observed_at_ns)
            .min()
            .unwrap_or(sampled_at_ns);
        SNAPSHOTS.with_borrow_mut(|cache| {
            cache.insert(
                family,
                PublicMetricSnapshot {
                    sampled_at_ns,
                    truncated,
                    metrics,
                },
            );
        });
        Ok(())
    }

    #[must_use]
    pub fn snapshot(family: PublicMetricFamily) -> Option<PublicMetricSnapshot> {
        SNAPSHOTS.with_borrow(|cache| cache.get(&family).cloned())
    }
}

fn sample_order(left: &PublicMetricSample, right: &PublicMetricSample) -> std::cmp::Ordering {
    left.name
        .cmp(&right.name)
        .then(left.canister_id.cmp(&right.canister_id))
}
