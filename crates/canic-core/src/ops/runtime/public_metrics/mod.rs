//! Module: ops::runtime::public_metrics
//!
//! Responsibility: collect local aggregate counters and project the publication cache.
//! Does not own: timers, endpoint authorization, or application metric semantics.
//! Boundary: public query projection reads cached values only.

use crate::{
    InternalError,
    config::{Config, RoleRuntimeConfig},
    domain::{public_metrics::PublicMetricFamily, runtime::HealthStatus},
    dto::{
        metrics::MetricValue,
        page::{Page, PageRequest},
        public_status::{
            PublicHealth, PublicMetric, PublicMetricsRequest, PublicMetricsSnapshot,
            PublicSnapshotState,
        },
    },
    model::public_metrics::{
        PUBLIC_METRICS_STALE_AFTER_NS, PublicMetricSample, PublicMetricsCache,
    },
    ops::{
        ic::IcOps,
        runtime::{env::EnvOps, metrics},
    },
};
use std::collections::BTreeSet;

#[cfg(feature = "sharding")]
use crate::ops::storage::placement::sharding::ShardingRegistryOps;

/// Local sampling and public snapshot projection under immutable publication configuration.
pub struct PublicMetricsOps;

impl PublicMetricsOps {
    #[must_use]
    pub fn enabled() -> BTreeSet<PublicMetricFamily> {
        RoleRuntimeConfig::try_get()
            .map(|config| config.public_metrics.clone())
            .or_else(|| {
                Config::get()
                    .ok()
                    .map(|config| config.public_metrics.clone())
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn health() -> PublicHealth {
        let now = IcOps::now_nanos();
        PublicHealth {
            canister_id: IcOps::canister_self(),
            role: EnvOps::canister_role().ok().map(|role| role.to_string()),
            health: HealthStatus::Healthy,
            observed_at_ns: now,
        }
    }

    #[must_use]
    pub fn read(request: PublicMetricsRequest) -> PublicMetricsSnapshot {
        Self::project(request, &Self::enabled(), IcOps::now_nanos())
    }

    fn project(
        request: PublicMetricsRequest,
        enabled: &BTreeSet<PublicMetricFamily>,
        now_ns: u64,
    ) -> PublicMetricsSnapshot {
        let snapshot = enabled
            .contains(&request.family)
            .then(|| PublicMetricsCache::snapshot(request.family))
            .flatten();
        let state = if !enabled.contains(&request.family) {
            PublicSnapshotState::Disabled
        } else if let Some(snapshot) = &snapshot {
            if now_ns.saturating_sub(snapshot.sampled_at_ns) > PUBLIC_METRICS_STALE_AFTER_NS {
                PublicSnapshotState::Stale
            } else {
                PublicSnapshotState::Fresh
            }
        } else {
            PublicSnapshotState::Unavailable
        };
        let sampled_at_ns = snapshot.as_ref().map(|s| s.sampled_at_ns);
        let truncated = snapshot.as_ref().is_some_and(|s| s.truncated);
        let rows = snapshot.map_or_else(Vec::new, |s| {
            s.metrics
                .into_iter()
                .map(|row| PublicMetric {
                    name: row.name,
                    canister_id: row.canister_id,
                    value: row.value,
                    unit: row.unit,
                })
                .collect()
        });
        PublicMetricsSnapshot {
            family: request.family,
            state,
            sampled_at_ns,
            stale_after_ns: PUBLIC_METRICS_STALE_AFTER_NS,
            truncated,
            metrics: page(rows, request.page),
        }
    }

    pub fn record_application(metrics: Vec<PublicMetric>) -> Result<(), InternalError> {
        if !Self::enabled().contains(&PublicMetricFamily::Application) {
            return Ok(());
        }
        let rows = metrics
            .into_iter()
            .map(|row| PublicMetricSample {
                name: row.name,
                canister_id: row.canister_id,
                value: row.value,
                unit: row.unit,
            })
            .collect();
        PublicMetricsCache::replace(PublicMetricFamily::Application, IcOps::now_nanos(), rows)
    }

    pub fn sample_family(family: PublicMetricFamily, now: u64) -> Result<(), InternalError> {
        let rows = match family {
            PublicMetricFamily::Application => return Ok(()),
            PublicMetricFamily::Cycles => vec![PublicMetricSample {
                name: "balance".into(),
                canister_id: Some(IcOps::canister_self()),
                value: IcOps::canister_cycle_balance().to_u128(),
                unit: "cycles".into(),
            }],
            PublicMetricFamily::Operations => operation_metrics(),
            PublicMetricFamily::Performance => performance_metrics(),
            PublicMetricFamily::ShardOccupancy => shard_metrics(),
        };
        PublicMetricsCache::replace(family, now, rows)
    }
}

fn page(rows: Vec<PublicMetric>, request: PageRequest) -> Page<PublicMetric> {
    let total = u64::try_from(rows.len()).unwrap_or(u64::MAX);
    let start = usize::try_from(request.offset.min(total)).unwrap_or(rows.len());
    let limit = usize::try_from(request.limit.min(total)).unwrap_or(rows.len());
    Page {
        entries: rows.into_iter().skip(start).take(limit).collect(),
        total,
    }
}

fn operation_metrics() -> Vec<PublicMetricSample> {
    metrics::core_entries()
        .into_iter()
        .flat_map(|row| {
            let name = row.labels.join(".");
            let amount_unit = if row.labels.iter().any(|label| label == "amount_e8s") {
                "icp_e8s"
            } else {
                "cycles"
            };
            match row.value {
                MetricValue::Count(value) => vec![PublicMetricSample {
                    name,
                    canister_id: row.principal,
                    value: u128::from(value),
                    unit: "count".into(),
                }],
                MetricValue::U128(value) => vec![PublicMetricSample {
                    name,
                    canister_id: row.principal,
                    value,
                    unit: amount_unit.into(),
                }],
                MetricValue::CountAndU64 { count, value_u64 } => vec![
                    PublicMetricSample {
                        name: format!("{name}.count"),
                        canister_id: row.principal,
                        value: u128::from(count),
                        unit: "count".into(),
                    },
                    PublicMetricSample {
                        name,
                        canister_id: row.principal,
                        value: u128::from(value_u64),
                        unit: "value".into(),
                    },
                ],
            }
        })
        .collect()
}

fn performance_metrics() -> Vec<PublicMetricSample> {
    metrics::runtime_entries()
        .into_iter()
        .filter(|row| row.labels.first().is_some_and(|label| label == "perf"))
        .flat_map(|row| {
            let MetricValue::CountAndU64 { count, value_u64 } = row.value else {
                return Vec::new();
            };
            let name = row.labels.join(".");
            vec![
                PublicMetricSample {
                    name: format!("{name}.calls"),
                    canister_id: None,
                    value: u128::from(count),
                    unit: "count".into(),
                },
                PublicMetricSample {
                    name,
                    canister_id: None,
                    value: u128::from(value_u64),
                    unit: "instructions".into(),
                },
            ]
        })
        .collect()
}

#[cfg(feature = "sharding")]
fn shard_metrics() -> Vec<PublicMetricSample> {
    ShardingRegistryOps::registry_data()
        .entries
        .into_iter()
        .flat_map(|row| {
            vec![
                PublicMetricSample {
                    name: format!("{}.assigned", row.entry.pool),
                    canister_id: Some(row.pid),
                    value: u128::from(row.entry.count),
                    unit: "assignments".into(),
                },
                PublicMetricSample {
                    name: format!("{}.capacity", row.entry.pool),
                    canister_id: Some(row.pid),
                    value: u128::from(row.entry.capacity),
                    unit: "assignments".into(),
                },
            ]
        })
        .collect()
}
#[cfg(not(feature = "sharding"))]
const fn shard_metrics() -> Vec<PublicMetricSample> {
    Vec::new()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "sharding")]
    use crate::ids::CanisterRole;
    use crate::model::public_metrics::MAX_PUBLIC_METRICS;

    fn request(family: PublicMetricFamily) -> PublicMetricsRequest {
        PublicMetricsRequest {
            family,
            page: PageRequest {
                limit: 1_000,
                offset: 0,
            },
        }
    }
    fn sample(value: u128) -> PublicMetricSample {
        PublicMetricSample {
            name: format!("assigned.{value:04}"),
            canister_id: None,
            value,
            unit: "assignments".into(),
        }
    }

    #[test]
    fn publication_is_disabled_even_when_a_cached_snapshot_exists() {
        let family = PublicMetricFamily::Cycles;
        PublicMetricsCache::replace(family, 10, vec![sample(7)]).unwrap();
        let result = PublicMetricsOps::project(request(family), &BTreeSet::new(), 10);
        assert_eq!(result.state, PublicSnapshotState::Disabled);
        assert_eq!(result.sampled_at_ns, None);
        assert!(result.metrics.entries.is_empty());
    }

    #[test]
    fn reads_preserve_sample_time_and_report_staleness_without_refresh() {
        let family = PublicMetricFamily::Performance;
        let enabled = BTreeSet::from([family]);
        let missing = PublicMetricsOps::project(request(family), &enabled, 10);
        assert_eq!(missing.state, PublicSnapshotState::Unavailable);
        PublicMetricsCache::replace(family, 10, vec![sample(3)]).unwrap();
        let fresh = PublicMetricsOps::project(request(family), &enabled, 10);
        assert_eq!(fresh.state, PublicSnapshotState::Fresh);
        let stale = PublicMetricsOps::project(
            request(family),
            &enabled,
            11 + PUBLIC_METRICS_STALE_AFTER_NS,
        );
        assert_eq!(stale.state, PublicSnapshotState::Stale);
        assert_eq!(stale.sampled_at_ns, Some(10));
        assert_eq!(stale.metrics.entries, fresh.metrics.entries);
        assert_eq!(
            PublicMetricsCache::snapshot(family).unwrap().sampled_at_ns,
            10
        );
    }

    #[test]
    fn publication_selection_is_exact_and_pages_are_bounded() {
        let family = PublicMetricFamily::ShardOccupancy;
        let enabled = BTreeSet::from([family]);
        PublicMetricsCache::replace(
            family,
            20,
            (0..=MAX_PUBLIC_METRICS)
                .map(|v| sample(v as u128))
                .collect(),
        )
        .unwrap();
        let all = PublicMetricsOps::project(request(family), &enabled, 20);
        assert!(all.truncated);
        assert_eq!(all.metrics.entries.len(), MAX_PUBLIC_METRICS);
        let mut req = request(family);
        req.page = PageRequest {
            limit: 2,
            offset: 1,
        };
        let page = PublicMetricsOps::project(req, &enabled, 20);
        assert_eq!(page.metrics.entries, all.metrics.entries[1..3]);
        assert_eq!(
            PublicMetricsOps::project(request(PublicMetricFamily::Operations), &enabled, 20).state,
            PublicSnapshotState::Disabled
        );
    }

    #[test]
    #[cfg(feature = "sharding")]
    fn shard_occupancy_samples_assignments_and_capacity_without_keys() {
        let shard = crate::cdk::types::Principal::from_slice(&[42; 29]);
        ShardingRegistryOps::clear_for_test();
        ShardingRegistryOps::create(shard, "demo", 0, &CanisterRole::new("shard"), 4, 0).unwrap();
        ShardingRegistryOps::assign("demo", "private-key-a", shard).unwrap();
        ShardingRegistryOps::assign("demo", "private-key-b", shard).unwrap();
        let family = PublicMetricFamily::ShardOccupancy;
        let enabled = BTreeSet::from([family]);
        PublicMetricsOps::sample_family(family, 10).unwrap();
        let snapshot = PublicMetricsOps::project(request(family), &enabled, 10);
        assert_eq!(
            snapshot.metrics.entries,
            vec![
                PublicMetric {
                    name: "demo.assigned".into(),
                    canister_id: Some(shard),
                    value: 2,
                    unit: "assignments".into()
                },
                PublicMetric {
                    name: "demo.capacity".into(),
                    canister_id: Some(shard),
                    value: 4,
                    unit: "assignments".into()
                },
            ]
        );
        ShardingRegistryOps::release("demo", "private-key-a").unwrap();
        let cached = PublicMetricsOps::project(request(family), &enabled, 11);
        assert_eq!(cached.metrics.entries, snapshot.metrics.entries);
        PublicMetricsOps::sample_family(family, 12).unwrap();
        let refreshed = PublicMetricsOps::project(request(family), &enabled, 12);
        assert_eq!(refreshed.metrics.entries[0].value, 1);
        assert_eq!(refreshed.sampled_at_ns, Some(12));
        ShardingRegistryOps::clear_for_test();
    }

    #[test]
    fn rejected_sample_preserves_previous_snapshot() {
        let family = PublicMetricFamily::Application;
        PublicMetricsCache::replace(family, 10, vec![sample(1)]).unwrap();
        let mut invalid = sample(2);
        invalid.name.clear();
        assert_eq!(
            PublicMetricsCache::replace(family, 20, vec![invalid])
                .unwrap_err()
                .code(),
            crate::diagnostics::codes::REQUEST_INVALID
        );
        assert_eq!(
            PublicMetricsCache::snapshot(family).unwrap().sampled_at_ns,
            10
        );
    }
}
