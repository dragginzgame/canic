//! Module: ops::runtime::public_metrics
//!
//! Responsibility: collect local aggregate counters and project the publication cache.
//! Does not own: timers, endpoint authorization, or application metric semantics.
//! Boundary: public query projection reads cached values only.

mod process;

use crate::{
    InternalError,
    config::{Config, RoleRuntimeConfig},
    domain::public_metrics::PublicMetricFamily,
    dto::{
        metrics::MetricValue,
        page::{Page, PageRequest},
        public_status::{
            PublicCounterDelta, PublicHealth, PublicHealthStatus, PublicHistoryPoint,
            PublicHistoryRequest, PublicHistorySnapshot, PublicMetric, PublicMetricKind,
            PublicMetricsRequest, PublicMetricsSnapshot, PublicSnapshotState,
        },
    },
    model::public_metrics::{
        MAX_HISTORY_BYTES, MAX_HISTORY_SERIES, MAX_PUBLIC_METRIC_TEXT_BYTES, MAX_PUBLIC_METRICS,
        PUBLIC_HISTORY_RETENTION_NS, PUBLIC_HISTORY_SLOTS, PUBLIC_METRICS_CADENCE_NS,
        PUBLIC_METRICS_STALE_AFTER_NS, PublicHistoryCache, PublicMetricSample, PublicMetricsCache,
    },
    ops::{
        ic::IcOps,
        runtime::{env::EnvOps, metrics},
    },
};
use std::{cell::Cell, collections::BTreeSet};

thread_local! {
    static APPLICATION_SAMPLER: Cell<Option<ApplicationMetricsSampler>> = const { Cell::new(None) };
}

#[cfg(feature = "sharding")]
use crate::ops::storage::placement::sharding::ShardingRegistryOps;

/// One synchronous aggregate provider composed by application lifecycle code.
/// The provider owns bounded source collection; Canic owns the timer and history.
#[derive(Clone, Copy)]
pub struct ApplicationMetricsSampler {
    collect: fn() -> Result<Vec<PublicMetric>, crate::dto::error::Error>,
}

impl ApplicationMetricsSampler {
    /// Wrap a provider that reads bounded counters and preserves source time and reset windows.
    #[must_use]
    pub const fn new(collect: fn() -> Result<Vec<PublicMetric>, crate::dto::error::Error>) -> Self {
        Self { collect }
    }
}

/// Local sampling and public snapshot projection under immutable publication configuration.
pub struct PublicMetricsOps;

impl PublicMetricsOps {
    /// Install the sole synchronous composition callback, with no timer or database ownership.
    pub fn set_application_sampler(sample: Option<ApplicationMetricsSampler>) {
        APPLICATION_SAMPLER.set(sample);
    }

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
            health: PublicHealthStatus::Responding,
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
                    observed_at_ns: row.observed_at_ns,
                    kind: row.kind,
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

    /// Expire heap history from the update-side timer, never from a public query.
    pub fn expire_history(now_ns: u64) {
        PublicHistoryCache::expire(now_ns);
    }

    /// Read one bounded cached series without invoking any producer.
    #[must_use]
    pub fn history(request: PublicHistoryRequest) -> PublicHistorySnapshot {
        let mut snapshot = Self::project_history(request, &Self::enabled(), IcOps::now_nanos());
        snapshot.canister_version = ic_cdk::api::canister_version();
        snapshot
    }

    fn project_history(
        request: PublicHistoryRequest,
        enabled: &BTreeSet<PublicMetricFamily>,
        now_ns: u64,
    ) -> PublicHistorySnapshot {
        let selected = enabled.contains(&request.family);
        let valid_name = request.name.len() <= MAX_PUBLIC_METRIC_TEXT_BYTES;
        let series = (selected && valid_name)
            .then(|| PublicHistoryCache::series(request.family, request.name, request.canister_id))
            .flatten();
        let slot = now_ns / PUBLIC_METRICS_CADENCE_NS;
        let mut points: Vec<_> = series.as_ref().map_or_else(Vec::new, |series| {
            series
                .slots
                .iter()
                .filter(|point| {
                    point.slot <= slot && slot - point.slot < PUBLIC_HISTORY_SLOTS as u64
                })
                .copied()
                .collect()
        });
        points.sort_by_key(|point| point.slot);
        let state = if !selected {
            PublicSnapshotState::Disabled
        } else if let Some(point) = points.last() {
            if now_ns.saturating_sub(point.observed_at_ns) > PUBLIC_METRICS_STALE_AFTER_NS {
                PublicSnapshotState::Stale
            } else {
                PublicSnapshotState::Fresh
            }
        } else {
            PublicSnapshotState::Unavailable
        };
        let coverage_start_ns = points
            .first()
            .map(|point| point.slot * PUBLIC_METRICS_CADENCE_NS);
        let total = points.len() as u64;
        let entries = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let delta = index
                    .checked_sub(1)
                    .and_then(|previous| counter_delta(&points[previous], point));
                PublicHistoryPoint {
                    delta,
                    slot_start_ns: point.slot * PUBLIC_METRICS_CADENCE_NS,
                    observed_at_ns: point.observed_at_ns,
                    value: point.value,
                    kind: point.kind,
                }
            })
            .skip(usize::try_from(request.page.offset.min(total)).unwrap_or(PUBLIC_HISTORY_SLOTS))
            .take(
                usize::try_from(request.page.limit.min(PUBLIC_HISTORY_SLOTS as u64))
                    .unwrap_or(PUBLIC_HISTORY_SLOTS),
            )
            .collect();
        PublicHistorySnapshot {
            state,
            unit: series.map(|series| series.unit),
            heap_started_at_ns: selected
                .then(PublicHistoryCache::heap_started_at_ns)
                .flatten(),
            canister_version: 0,
            coverage_start_ns,
            cadence_ns: PUBLIC_METRICS_CADENCE_NS,
            retention_ns: PUBLIC_HISTORY_RETENTION_NS,
            stale_after_ns: PUBLIC_METRICS_STALE_AFTER_NS,
            truncated: selected && PublicHistoryCache::truncated(),
            series_limit: MAX_HISTORY_SERIES as u64,
            byte_limit: MAX_HISTORY_BYTES as u64,
            reserved_bytes: if selected {
                PublicHistoryCache::reserved_bytes() as u64
            } else {
                0
            },
            points: Page { entries, total },
        }
    }

    pub fn record_application(metrics: Vec<PublicMetric>) -> Result<(), InternalError> {
        if !Self::enabled().contains(&PublicMetricFamily::Application) {
            return Ok(());
        }
        let rows = metrics.into_iter().map(|row| PublicMetricSample {
            name: row.name,
            canister_id: row.canister_id,
            value: row.value,
            unit: row.unit,
            observed_at_ns: row.observed_at_ns,
            kind: row.kind,
        });
        PublicMetricsCache::replace(PublicMetricFamily::Application, IcOps::now_nanos(), rows)
    }

    pub fn sample_family(family: PublicMetricFamily, now: u64) -> Result<(), InternalError> {
        let mut rows = match family {
            PublicMetricFamily::Application => {
                if let Some(sample) = APPLICATION_SAMPLER.get() {
                    let metrics = (sample.collect)().map_err(|_| InternalError::invalid_input())?;
                    return Self::record_application(metrics);
                }
                return Ok(());
            }
            PublicMetricFamily::Cycles => vec![PublicMetricSample {
                name: "balance".into(),
                canister_id: Some(IcOps::canister_self()),
                value: IcOps::canister_cycle_balance().to_u128(),
                unit: "cycles".into(),
                observed_at_ns: 0,
                kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
            }],
            PublicMetricFamily::Operations => {
                let mut rows = process::operations()?;
                rows.extend(operation_metrics()?);
                rows
            }
            PublicMetricFamily::Performance => {
                let mut rows = process::memory();
                rows.extend(performance_metrics()?);
                rows
            }
            PublicMetricFamily::ShardOccupancy => shard_metrics(),
        };
        for row in &mut rows {
            row.observed_at_ns = now;
            // Timers expose lifetime summaries without a per-registration reset identity.
            // Keep these raw observations out of counter delta/rate calculations.
            let counter = (family == PublicMetricFamily::Operations
                && !row.name.starts_with("cycles_funding.icp_refill.")
                && !row.name.starts_with("timer."))
                || (family == PublicMetricFamily::Performance
                    && !row.name.starts_with("perf.timer.")
                    && !row.name.starts_with("memory."));
            if counter {
                row.kind = PublicMetricKind::Counter {
                    window_id: 0,
                    saturated: if matches!(row.unit.as_str(), "cycles" | "icp_e8s") {
                        row.value == u128::MAX
                    } else {
                        row.value == u128::from(u64::MAX)
                    },
                };
            }
        }
        PublicMetricsCache::replace(family, now, rows)
    }
}

fn counter_delta(
    previous: &crate::model::public_metrics::PublicHistorySample,
    current: &crate::model::public_metrics::PublicHistorySample,
) -> Option<PublicCounterDelta> {
    let PublicMetricKind::Counter {
        window_id,
        saturated: false,
    } = previous.kind
    else {
        return None;
    };
    if current.kind
        != (PublicMetricKind::Counter {
            window_id,
            saturated: false,
        })
        || previous.slot.checked_add(1) != Some(current.slot)
    {
        return None;
    }
    let elapsed_ns = current
        .observed_at_ns
        .checked_sub(previous.observed_at_ns)
        .filter(|elapsed| *elapsed > 0)?;
    Some(PublicCounterDelta {
        amount: current.value.checked_sub(previous.value)?,
        elapsed_ns,
    })
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

// Validate the complete series identity before allocating its formatted name.
fn metric_name(labels: &[String], suffix_bytes: usize) -> Result<String, InternalError> {
    let bytes = labels.iter().try_fold(
        suffix_bytes + labels.len().saturating_sub(1),
        |bytes, label| bytes.checked_add(label.len()),
    );
    if bytes.is_none_or(|bytes| bytes > MAX_PUBLIC_METRIC_TEXT_BYTES) {
        return Err(InternalError::invalid_input());
    }
    Ok(labels.join("."))
}

fn operation_metrics() -> Result<Vec<PublicMetricSample>, InternalError> {
    metrics::bounded_core_entries(MAX_PUBLIC_METRICS + 1)?
        .into_iter()
        .take(MAX_PUBLIC_METRICS + 1)
        .map(|row| {
            let suffix_bytes = if matches!(&row.value, MetricValue::CountAndU64 { .. }) {
                6
            } else {
                0
            };
            let name = metric_name(&row.labels, suffix_bytes)?;
            let amount_unit = if row.labels.iter().any(|label| label == "amount_e8s") {
                "icp_e8s"
            } else {
                "cycles"
            };
            Ok(match row.value {
                MetricValue::Count(value) => vec![PublicMetricSample {
                    name,
                    canister_id: row.principal,
                    value: u128::from(value),
                    unit: "count".into(),
                    observed_at_ns: 0,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                }],
                MetricValue::U128(value) => vec![PublicMetricSample {
                    name,
                    canister_id: row.principal,
                    value,
                    unit: amount_unit.into(),
                    observed_at_ns: 0,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                }],
                MetricValue::CountAndU64 { count, value_u64 } => vec![
                    PublicMetricSample {
                        name: format!("{name}.count"),
                        canister_id: row.principal,
                        value: u128::from(count),
                        unit: "count".into(),
                        observed_at_ns: 0,
                        kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                    },
                    PublicMetricSample {
                        name,
                        canister_id: row.principal,
                        value: u128::from(value_u64),
                        unit: "value".into(),
                        observed_at_ns: 0,
                        kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                    },
                ],
            })
        })
        .collect::<Result<Vec<_>, InternalError>>()
        .map(|rows| rows.into_iter().flatten().collect())
}

fn performance_metrics() -> Result<Vec<PublicMetricSample>, InternalError> {
    metrics::bounded_performance_entries(MAX_PUBLIC_METRICS / 2 + 1)?
        .into_iter()
        .take(MAX_PUBLIC_METRICS / 2 + 1)
        .map(|row| {
            let MetricValue::CountAndU64 { count, value_u64 } = row.value else {
                return Ok(Vec::new());
            };
            let name = metric_name(&row.labels, 6)?;
            Ok(vec![
                PublicMetricSample {
                    name: format!("{name}.calls"),
                    canister_id: None,
                    value: u128::from(count),
                    unit: "count".into(),
                    observed_at_ns: 0,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                },
                PublicMetricSample {
                    name,
                    canister_id: None,
                    value: u128::from(value_u64),
                    unit: "instructions".into(),
                    observed_at_ns: 0,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                },
            ])
        })
        .collect::<Result<Vec<_>, InternalError>>()
        .map(|rows| rows.into_iter().flatten().collect())
}

#[cfg(feature = "sharding")]
fn shard_metrics() -> Vec<PublicMetricSample> {
    ShardingRegistryOps::bounded_registry_entries(MAX_PUBLIC_METRICS / 2 + 1)
        .into_iter()
        .flat_map(|row| {
            vec![
                PublicMetricSample {
                    name: format!("{}.assigned", row.entry.pool),
                    canister_id: Some(row.pid),
                    value: u128::from(row.entry.count),
                    unit: "assignments".into(),
                    observed_at_ns: 0,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                },
                PublicMetricSample {
                    name: format!("{}.capacity", row.entry.pool),
                    canister_id: Some(row.pid),
                    value: u128::from(row.entry.capacity),
                    unit: "assignments".into(),
                    observed_at_ns: 0,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
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
    fn publish(
        family: PublicMetricFamily,
        now: u64,
        rows: impl IntoIterator<Item = PublicMetricSample>,
    ) -> Result<(), InternalError> {
        PublicMetricsCache::replace(
            family,
            now,
            rows.into_iter().map(|mut row| {
                row.observed_at_ns = now;
                row
            }),
        )
    }

    fn sample(value: u128) -> PublicMetricSample {
        PublicMetricSample {
            name: format!("assigned.{value:04}"),
            canister_id: None,
            value,
            unit: "assignments".into(),
            observed_at_ns: 0,
            kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
        }
    }

    #[test]
    fn publication_is_disabled_even_when_a_cached_snapshot_exists() {
        let family = PublicMetricFamily::Cycles;
        publish(family, 10, vec![sample(7)]).unwrap();
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
        publish(family, 10, vec![sample(3)]).unwrap();
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
        publish(
            family,
            20,
            (0..=MAX_PUBLIC_METRICS).map(|v| sample(v as u128)),
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
                    unit: "assignments".into(),
                    observed_at_ns: 10,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
                },
                PublicMetric {
                    name: "demo.capacity".into(),
                    canister_id: Some(shard),
                    value: 4,
                    unit: "assignments".into(),
                    observed_at_ns: 10,
                    kind: crate::domain::public_metrics::PublicMetricKind::Gauge,
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
        publish(family, 10, vec![sample(1)]).unwrap();
        let mut invalid = sample(2);
        invalid.name.clear();
        assert_eq!(
            publish(family, 20, vec![invalid]).unwrap_err().code(),
            crate::diagnostics::codes::REQUEST_INVALID
        );
        assert_eq!(
            PublicMetricsCache::snapshot(family).unwrap().sampled_at_ns,
            10
        );
    }
    #[test]
    fn cache_consumes_only_one_bounded_prefix_and_reports_truncation() {
        let consumed = std::cell::Cell::new(0);
        publish(
            PublicMetricFamily::Application,
            10,
            (0..).map(|value| {
                consumed.set(consumed.get() + 1);
                sample(value)
            }),
        )
        .unwrap();
        assert_eq!(consumed.get(), MAX_PUBLIC_METRICS + 1);
        let snapshot = PublicMetricsCache::snapshot(PublicMetricFamily::Application).unwrap();
        assert!(snapshot.truncated);
        assert_eq!(snapshot.metrics.len(), MAX_PUBLIC_METRICS);
    }

    #[test]
    fn performance_sampling_is_bounded_and_independent_of_recording_order() {
        let family = PublicMetricFamily::Performance;
        crate::perf::reset();
        for value in (0..1024).rev() {
            crate::perf::record_checkpoint("bounded", &format!("sample_{value:04}"), value);
        }
        PublicMetricsOps::sample_family(family, 10).unwrap();
        let first = PublicMetricsOps::project(request(family), &BTreeSet::from([family]), 10);
        assert!(first.truncated);
        assert_eq!(first.metrics.entries.len(), MAX_PUBLIC_METRICS);
        assert_eq!(
            first.metrics.entries[0].name,
            "perf.checkpoint.bounded.sample_0000"
        );
        crate::perf::reset();
        for value in 0..1024 {
            crate::perf::record_checkpoint("bounded", &format!("sample_{value:04}"), value);
        }
        PublicMetricsOps::sample_family(family, 20).unwrap();
        let second = PublicMetricsOps::project(request(family), &BTreeSet::from([family]), 20);
        for (first, second) in first.metrics.entries.iter().zip(&second.metrics.entries) {
            assert_eq!(first.name, second.name);
            assert_eq!(first.value, second.value);
            assert_eq!(first.kind, second.kind);
            assert_eq!(first.observed_at_ns, 10);
            assert_eq!(second.observed_at_ns, 20);
        }
        crate::perf::reset();
    }

    #[test]
    #[cfg(feature = "sharding")]
    fn shard_sampling_bounds_registry_visits_and_retained_rows() {
        ShardingRegistryOps::clear_for_test();
        for value in 0_u32..300 {
            let shard = crate::cdk::types::Principal::from_slice(&value.to_be_bytes());
            ShardingRegistryOps::create(shard, "bounded", value, &CanisterRole::new("shard"), 4, 0)
                .unwrap();
        }
        assert_eq!(
            ShardingRegistryOps::bounded_registry_entries(129).len(),
            129
        );
        PublicMetricsOps::sample_family(PublicMetricFamily::ShardOccupancy, 10).unwrap();
        let snapshot = PublicMetricsCache::snapshot(PublicMetricFamily::ShardOccupancy).unwrap();
        assert!(snapshot.truncated);
        assert_eq!(snapshot.metrics.len(), MAX_PUBLIC_METRICS);
        ShardingRegistryOps::clear_for_test();
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn history_reads_bound_pages_hide_disabled_data_and_expire_without_mutation() {
        let family = PublicMetricFamily::Cycles;
        for slot in [1, 2, 5] {
            PublicMetricsCache::replace(
                family,
                slot * PUBLIC_METRICS_CADENCE_NS,
                [PublicMetricSample {
                    name: "balance".into(),
                    canister_id: None,
                    value: slot.into(),
                    unit: "cycles".into(),
                    observed_at_ns: slot * PUBLIC_METRICS_CADENCE_NS,
                    kind: PublicMetricKind::Gauge,
                }],
            )
            .unwrap();
        }
        let request = PublicHistoryRequest {
            family,
            name: "balance".into(),
            canister_id: None,
            page: PageRequest {
                offset: 1,
                limit: u64::MAX,
            },
        };
        let enabled = BTreeSet::from([family]);
        let view = PublicMetricsOps::project_history(
            request.clone(),
            &enabled,
            5 * PUBLIC_METRICS_CADENCE_NS,
        );
        assert_eq!(view.points.total, 3);
        assert_eq!(
            view.points
                .entries
                .iter()
                .map(|point| point.value)
                .collect::<Vec<_>>(),
            [2, 5]
        );
        assert_eq!(view.coverage_start_ns, Some(PUBLIC_METRICS_CADENCE_NS));
        let disabled = PublicMetricsOps::project_history(
            request.clone(),
            &BTreeSet::new(),
            5 * PUBLIC_METRICS_CADENCE_NS,
        );
        assert_eq!(disabled.state, PublicSnapshotState::Disabled);
        assert!(disabled.points.entries.is_empty());
        assert_eq!(disabled.reserved_bytes, 0);
        let expired =
            PublicMetricsOps::project_history(request, &enabled, 400 * PUBLIC_METRICS_CADENCE_NS);
        assert_eq!(expired.state, PublicSnapshotState::Unavailable);
        assert!(expired.points.entries.is_empty());
        assert_eq!(expired.reserved_bytes, view.reserved_bytes);
        assert_eq!(
            PublicMetricsCache::snapshot(family).unwrap().sampled_at_ns,
            5 * PUBLIC_METRICS_CADENCE_NS
        );
    }
}

#[cfg(test)]
mod counter_tests {
    use super::*;
    use crate::model::public_metrics::PublicHistorySample;

    #[test]
    fn public_metrics_counter_deltas_require_adjacent_unsaturated_same_window_observations() {
        let first = PublicHistorySample {
            slot: 1,
            observed_at_ns: 10,
            value: 7,
            kind: PublicMetricKind::Counter {
                window_id: 4,
                saturated: false,
            },
        };
        let second = PublicHistorySample {
            slot: 2,
            observed_at_ns: 20,
            value: 12,
            ..first
        };
        assert_eq!(
            counter_delta(&first, &second),
            Some(PublicCounterDelta {
                amount: 5,
                elapsed_ns: 10
            })
        );
        for incompatible in [
            PublicHistorySample {
                kind: PublicMetricKind::Gauge,
                ..second
            },
            PublicHistorySample {
                kind: PublicMetricKind::Counter {
                    window_id: 5,
                    saturated: false,
                },
                ..second
            },
            PublicHistorySample {
                kind: PublicMetricKind::Counter {
                    window_id: 4,
                    saturated: true,
                },
                ..second
            },
            PublicHistorySample { slot: 3, ..second },
            PublicHistorySample {
                observed_at_ns: 10,
                ..second
            },
            PublicHistorySample { value: 1, ..second },
        ] {
            assert_eq!(counter_delta(&first, &incompatible), None);
        }
    }
}
