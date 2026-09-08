//! Module: model::public_metrics
//!
//! Responsibility: retain bounded ephemeral aggregate snapshots.
//! Does not own: collection, endpoint authorization, or wire conversion.
//! Boundary: caches may be rebuilt after restoration and never carry authority.

use crate::{cdk::types::Principal, domain::public_metrics::PublicMetricFamily};
use std::{cell::RefCell, collections::BTreeMap};

/// Maximum rows retained per family, independent of caller pagination.
pub const MAX_PUBLIC_METRICS: usize = 256;
/// Maximum bytes in a public metric name or unit.
pub const MAX_PUBLIC_METRIC_TEXT_BYTES: usize = 128;
/// Cached values older than five minutes are explicitly stale.
pub const PUBLIC_METRICS_STALE_AFTER_NS: u64 = 300_000_000_000;

/// One retained aggregate sample with no user or partition-key dimension.
#[derive(Clone, Debug)]
pub struct PublicMetricSample {
    pub name: String,
    pub canister_id: Option<Principal>,
    pub value: u128,
    pub unit: String,
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
        mut metrics: Vec<PublicMetricSample>,
    ) -> Result<(), crate::InternalError> {
        if metrics.iter().any(|row| {
            row.name.is_empty()
                || row.unit.is_empty()
                || row.name.len() > MAX_PUBLIC_METRIC_TEXT_BYTES
                || row.unit.len() > MAX_PUBLIC_METRIC_TEXT_BYTES
        }) {
            return Err(crate::InternalError::invalid_input());
        }
        metrics.sort_by(|a, b| a.name.cmp(&b.name).then(a.canister_id.cmp(&b.canister_id)));
        let truncated = metrics.len() > MAX_PUBLIC_METRICS;
        metrics.truncate(MAX_PUBLIC_METRICS);
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
