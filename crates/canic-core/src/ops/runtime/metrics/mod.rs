//! Module: ops::runtime::metrics
//!
//! Responsibility: project runtime metric families into public metric rows.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

pub mod access;
pub mod auth;
pub mod canister_ops;
pub mod cascade;
pub mod cycles_funding;
pub mod cycles_topup;
pub mod delegated_auth;
mod error;
pub mod icp_refill;
pub mod intent;
pub mod inter_canister_call;
pub mod lifecycle;
pub mod placement_index;
pub mod platform_call;
pub mod recording;
pub mod replay;
pub mod root_capability;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
pub mod system;
pub mod wasm_store;

use crate::{
    domain::metrics::MetricsKind,
    dto::metrics::{MetricEntry, MetricValue},
    ops::runtime::env::EnvOps,
    perf::{self, PerfKey},
};
use {
    access::AccessMetrics, auth::AuthMetrics, canister_ops::CanisterOpsMetrics,
    cascade::CascadeMetrics, cycles_funding::CyclesFundingMetrics,
    cycles_topup::CyclesTopupMetrics, delegated_auth::DelegatedAuthMetrics,
    icp_refill::IcpRefillMetrics, intent::IntentMetrics,
    inter_canister_call::InterCanisterCallMetrics, lifecycle::LifecycleMetrics,
    placement_index::PlacementIndexMetrics, platform_call::PlatformCallMetrics,
    replay::ReplayMetrics, root_capability::RootCapabilityMetrics, scaling::ScalingMetrics,
    wasm_store::WasmStoreMetrics,
};

#[cfg(feature = "sharding")]
use sharding::ShardingMetrics;

#[cfg(test)]
use system::SystemMetrics;

/// Project one public metrics tier into the unified row shape.
#[must_use]
pub fn entries(kind: MetricsKind) -> Vec<MetricEntry> {
    match kind {
        MetricsKind::Core => core_entries(),
        MetricsKind::Placement => placement_entries(),
        MetricsKind::Platform => platform_entries(),
        MetricsKind::Runtime => runtime_entries(),
        MetricsKind::Security => security_entries(),
        MetricsKind::Storage => storage_entries(),
    }
}

#[must_use]
pub fn core_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("lifecycle", lifecycle_entries());
    entries.extend(prefix_entries("canister_ops", canister_ops_entries()));
    entries.extend(prefix_entries("cycles_funding", cycles_funding_entries()));
    entries.extend(prefix_entries("cycles_topup", cycles_topup_entries()));
    entries
}

/// Project at most `limit` inputs from each operation owner before formatting.
pub(super) fn bounded_core_entries(limit: usize) -> Result<Vec<MetricEntry>, crate::InternalError> {
    let mut rows = prefix_entries(
        "lifecycle",
        lifecycle_entries_from(LifecycleMetrics::bounded_snapshot(limit)),
    );
    rows.extend(prefix_entries(
        "canister_ops",
        canister_ops_entries_from(CanisterOpsMetrics::bounded_snapshot(limit)?),
    ));
    rows.extend(prefix_entries(
        "cycles_funding",
        cycles_funding_entries_from(limit),
    ));
    if EnvOps::is_root() {
        rows.extend(prefix_entries(
            "cycles_funding",
            icp_refill::entries_from_snapshot(
                &crate::ops::storage::icp_refill::IcpRefillRecordOps::bounded_metric_snapshot(
                    limit,
                ),
            ),
        ));
    }
    rows.extend(prefix_entries(
        "cycles_topup",
        cycles_topup_entries_from(CyclesTopupMetrics::bounded_snapshot(limit)),
    ));
    Ok(rows)
}

#[must_use]
pub fn placement_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("cascade", cascade_entries());
    entries.extend(prefix_entries("placement_index", placement_index_entries()));
    entries.extend(prefix_entries("scaling", scaling_entries()));
    #[cfg(feature = "sharding")]
    entries.extend(prefix_entries("sharding", sharding_entries()));
    entries
}

#[must_use]
pub fn platform_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("platform_call", platform_call_entries());
    entries.extend(prefix_entries(
        "inter_canister_call",
        inter_canister_call_entries(),
    ));
    entries
}

#[must_use]
pub fn runtime_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("intent", intent_entries());
    if let Ok(timer_inventory) = ic_timers::timer_inventory() {
        let timer_snapshots = timer_inventory.timers();
        entries.extend(prefix_entries("perf", perf_entries(timer_snapshots)));
        let mut timer_metrics = vec![timer_inventory_availability(true)];
        timer_metrics.extend(timer_entries(timer_snapshots));
        entries.extend(prefix_entries("timer", timer_metrics));
    } else {
        entries.extend(prefix_entries("perf", perf_entries(&[])));
        entries.extend(prefix_entries(
            "timer",
            vec![timer_inventory_availability(false)],
        ));
    }
    entries
}

fn timer_inventory_availability(available: bool) -> MetricEntry {
    MetricEntry {
        labels: vec!["inventory".to_string(), "available".to_string()],
        principal: None,
        value: MetricValue::Count(u64::from(available)),
    }
}

#[must_use]
pub fn security_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("access", access_entries());
    entries.extend(prefix_entries("auth", auth_entries()));
    entries.extend(prefix_entries("delegated_auth", delegated_auth_entries()));
    entries.extend(prefix_entries("replay", replay_entries()));
    entries.extend(prefix_entries("root_capability", root_capability_entries()));
    entries
}

#[must_use]
pub fn storage_entries() -> Vec<MetricEntry> {
    prefix_entries("wasm_store", wasm_store_entries())
}

#[must_use]
fn prefix_entries(family: &'static str, entries: Vec<MetricEntry>) -> Vec<MetricEntry> {
    entries
        .into_iter()
        .map(|mut entry| {
            entry.labels.insert(0, family.to_string());
            entry
        })
        .collect()
}

#[cfg(test)]
pub fn reset_for_tests() {
    AccessMetrics::reset();
    AuthMetrics::reset();
    CanisterOpsMetrics::reset();
    CascadeMetrics::reset();
    CyclesFundingMetrics::reset();
    CyclesTopupMetrics::reset();
    DelegatedAuthMetrics::reset();
    PlacementIndexMetrics::reset();
    PlatformCallMetrics::reset();
    InterCanisterCallMetrics::reset();
    IntentMetrics::reset();
    LifecycleMetrics::reset();
    ReplayMetrics::reset();
    RootCapabilityMetrics::reset();
    ScalingMetrics::reset();
    #[cfg(feature = "sharding")]
    ShardingMetrics::reset();
    SystemMetrics::reset();
    WasmStoreMetrics::reset();
    perf::reset();
}

/// Project low-cardinality platform call outcome counters into public metrics rows.
#[must_use]
fn platform_call_entries() -> Vec<MetricEntry> {
    PlatformCallMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.surface.metric_label().to_string(),
                key.mode.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project intent reservation counters into the unified public metrics row shape.
#[must_use]
fn intent_entries() -> Vec<MetricEntry> {
    IntentMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.surface.metric_label().to_string(),
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project replay safety counters into the unified public metrics row shape.
#[must_use]
fn replay_entries() -> Vec<MetricEntry> {
    ReplayMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project auth runtime counters into the unified public metrics row shape.
#[must_use]
fn auth_entries() -> Vec<MetricEntry> {
    AuthMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.surface.metric_label().to_string(),
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project Placement Index counters into the unified public metrics row shape.
#[must_use]
fn placement_index_entries() -> Vec<MetricEntry> {
    PlacementIndexMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project scaling workflow counters into the unified public metrics row shape.
#[must_use]
fn scaling_entries() -> Vec<MetricEntry> {
    ScalingMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project sharding placement counters into the unified public metrics row shape.
#[cfg(feature = "sharding")]
#[must_use]
fn sharding_entries() -> Vec<MetricEntry> {
    ShardingMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project cascade counters into the unified public metrics row shape.
#[must_use]
fn cascade_entries() -> Vec<MetricEntry> {
    CascadeMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.snapshot.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project canister operation counters into the unified public metrics row shape.
#[must_use]
fn canister_ops_entries() -> Vec<MetricEntry> {
    canister_ops_entries_from(CanisterOpsMetrics::snapshot())
}

fn canister_ops_entries_from(
    snapshot: Vec<(canister_ops::CanisterOpsMetricKey, u64)>,
) -> Vec<MetricEntry> {
    snapshot
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.role,
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project wasm-store operation counters into the unified public metrics row shape.
#[must_use]
fn wasm_store_entries() -> Vec<MetricEntry> {
    WasmStoreMetrics::snapshot()
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.operation.metric_label().to_string(),
                key.source.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
                key.reason.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project lifecycle counters into the unified public metrics row shape.
#[must_use]
fn lifecycle_entries() -> Vec<MetricEntry> {
    lifecycle_entries_from(LifecycleMetrics::snapshot())
}

fn lifecycle_entries_from(snapshot: Vec<(lifecycle::LifecycleMetricKey, u64)>) -> Vec<MetricEntry> {
    snapshot
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                key.phase.metric_label().to_string(),
                key.role.metric_label().to_string(),
                key.stage.metric_label().to_string(),
                key.outcome.metric_label().to_string(),
            ],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project inter-canister call counters into the unified public metrics row shape.
#[must_use]
fn inter_canister_call_entries() -> Vec<MetricEntry> {
    InterCanisterCallMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![key.method],
            principal: Some(key.target),
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project timer counters into the unified public metrics row shape.
#[must_use]
fn timer_entries(snapshots: &[ic_timers::TimerSnapshot]) -> Vec<MetricEntry> {
    snapshots
        .iter()
        .flat_map(|snapshot| {
            let identity = snapshot.identity();
            timer_rows(
                vec![
                    snapshot.policy().label().to_string(),
                    identity.owner().to_string(),
                    identity.subsystem().to_string(),
                    identity.name().to_string(),
                ],
                snapshot.observability().counters().work_started(),
                snapshot.latest_armed_delay_ns(),
            )
        })
        .collect()
}

fn timer_rows(
    mut labels: Vec<String>,
    work_started: u64,
    latest_delay_ns: Option<u64>,
) -> Vec<MetricEntry> {
    labels.push("work_started".into());
    let mut rows = vec![MetricEntry {
        labels: labels.clone(),
        principal: None,
        value: MetricValue::Count(work_started),
    }];
    if let Some(delay) = latest_delay_ns {
        labels.pop();
        labels.push("latest_armed_delay_ms".into());
        rows.push(MetricEntry {
            labels,
            principal: None,
            value: MetricValue::U128(u128::from(delay / 1_000_000)),
        });
    }
    rows
}

/// Project access-denial counters into the unified public metrics row shape.
#[must_use]
fn access_entries() -> Vec<MetricEntry> {
    AccessMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![key.endpoint, key.kind.as_str().to_string(), key.predicate],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project delegated-auth counters into the unified public metrics row shape.
#[must_use]
fn delegated_auth_entries() -> Vec<MetricEntry> {
    let mut entries: Vec<_> = DelegatedAuthMetrics::snapshot()
        .into_iter()
        .map(|(authority, count)| MetricEntry {
            labels: vec!["delegated_auth_authority".to_string()],
            principal: Some(authority),
            value: MetricValue::Count(count),
        })
        .collect();

    entries.extend(
        DelegatedAuthMetrics::event_snapshot()
            .into_iter()
            .map(|(key, count)| MetricEntry {
                labels: vec![
                    key.operation.metric_label().to_string(),
                    key.outcome.metric_label().to_string(),
                    key.reason.metric_label().to_string(),
                ],
                principal: None,
                value: MetricValue::Count(count),
            }),
    );

    entries
}

/// Project root-capability counters into the unified public metrics row shape.
#[must_use]
fn root_capability_entries() -> Vec<MetricEntry> {
    RootCapabilityMetrics::snapshot()
        .into_iter()
        .map(
            |(capability, event_type, outcome, proof_mode, count)| MetricEntry {
                labels: vec![
                    capability.metric_label().to_string(),
                    event_type.metric_label().to_string(),
                    outcome.metric_label().to_string(),
                    proof_mode.metric_label().to_string(),
                ],
                principal: None,
                value: MetricValue::Count(count),
            },
        )
        .collect()
}

/// Project cycles-funding counters into the unified public metrics row shape.
#[must_use]
fn cycles_funding_entries() -> Vec<MetricEntry> {
    let mut entries = cycles_funding_entries_from(usize::MAX);
    if EnvOps::is_root() {
        entries.extend(icp_refill_entries());
    }
    entries
}

fn cycles_funding_entries_from(limit: usize) -> Vec<MetricEntry> {
    CyclesFundingMetrics::bounded_snapshot(limit)
        .into_iter()
        .map(|(metric, child_principal, reason, cycles)| MetricEntry {
            labels: reason.map_or_else(
                || vec![metric.metric_label().to_string()],
                |reason| {
                    vec![
                        metric.metric_label().to_string(),
                        reason.metric_label().to_string(),
                    ]
                },
            ),
            principal: child_principal,
            value: MetricValue::U128(cycles),
        })
        .collect()
}

/// Project auto-top-up decision counters into the unified public metrics row shape.
#[must_use]
fn cycles_topup_entries() -> Vec<MetricEntry> {
    cycles_topup_entries_from(CyclesTopupMetrics::snapshot())
}

fn cycles_topup_entries_from(
    snapshot: Vec<(cycles_topup::CyclesTopupMetricKey, u64)>,
) -> Vec<MetricEntry> {
    snapshot
        .into_iter()
        .map(|(metric, count)| MetricEntry {
            labels: vec![metric.metric_label().to_string()],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project ICP refill recovery records into bounded public funding metrics rows.
#[must_use]
fn icp_refill_entries() -> Vec<MetricEntry> {
    IcpRefillMetrics::entries()
}

/// Project perf counters into the unified public metrics row shape.
#[must_use]
fn perf_entries(timer_snapshots: &[ic_timers::TimerSnapshot]) -> Vec<MetricEntry> {
    perf_entries_from(perf::entries(), timer_snapshots)
}

fn perf_entries_from(
    snapshot: Vec<perf::PerfEntry>,
    timer_snapshots: &[ic_timers::TimerSnapshot],
) -> Vec<MetricEntry> {
    let mut entries = snapshot
        .into_iter()
        .map(|entry| MetricEntry {
            labels: perf_labels(entry.key),
            principal: None,
            value: measurement_value(entry.aggregate),
        })
        .collect::<Vec<_>>();
    entries.extend(timer_snapshots.iter().flat_map(|snapshot| {
        let performance = snapshot.observability().performance();
        [
            ("scheduler", performance.scheduler_instructions()),
            ("work", performance.work_instructions()),
        ]
        .into_iter()
        .map(|(phase, summary)| MetricEntry {
            labels: timer_perf_labels(snapshot.identity(), phase),
            principal: None,
            value: measurement_value(summary.aggregate()),
        })
    }));
    entries
}

const fn measurement_value(aggregate: ic_metrics::MeasurementAggregate) -> MetricValue {
    MetricValue::Measurement {
        count: aggregate.samples(),
        total: aggregate.total(),
        count_overflowed: aggregate.samples_overflowed(),
        total_overflowed: aggregate.total_overflowed(),
    }
}

// Both diagnostic and public views use the same source label contract.
pub(super) fn perf_labels(key: PerfKey) -> Vec<String> {
    match key {
        PerfKey::Endpoint { kind, name } => {
            vec!["endpoint".into(), kind.metric_label().into(), name]
        }
        PerfKey::Checkpoint { scope, label } => vec!["checkpoint".into(), scope, label],
    }
}

pub(super) fn timer_perf_labels(identity: &ic_timers::TimerIdentity, phase: &str) -> Vec<String> {
    vec![
        "timer".into(),
        identity.owner().into(),
        identity.subsystem().into(),
        identity.name().into(),
        phase.into(),
    ]
}

#[cfg(test)]
mod tests;
