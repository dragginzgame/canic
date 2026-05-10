pub mod access;
pub mod auth;
pub mod canister_ops;
pub mod cascade;
pub mod cycles_funding;
pub mod cycles_topup;
pub mod delegated_auth;
pub mod directory;
pub mod http;
pub mod intent;
pub mod inter_canister_call;
pub mod lifecycle;
pub mod management_call;
pub mod platform_call;
pub mod pool;
pub mod provisioning;
pub mod recording;
pub mod replay;
pub mod root_capability;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
pub mod system;
pub mod timer;
pub mod wasm_store;

use crate::{
    dto::metrics::{MetricEntry, MetricValue, MetricsKind},
    perf::{self, PerfKey},
};
use {
    access::AccessMetrics,
    auth::AuthMetrics,
    canister_ops::CanisterOpsMetrics,
    cascade::CascadeMetrics,
    cycles_funding::CyclesFundingMetrics,
    cycles_topup::CyclesTopupMetrics,
    delegated_auth::DelegatedAuthMetrics,
    directory::DirectoryMetrics,
    http::HttpMetrics,
    intent::IntentMetrics,
    inter_canister_call::InterCanisterCallMetrics,
    lifecycle::LifecycleMetrics,
    platform_call::PlatformCallMetrics,
    pool::PoolMetrics,
    replay::ReplayMetrics,
    root_capability::RootCapabilityMetrics,
    scaling::ScalingMetrics,
    timer::{TimerMetrics, TimerMode},
    wasm_store::WasmStoreMetrics,
};

#[cfg(feature = "sharding")]
use sharding::ShardingMetrics;

#[cfg(test)]
use {
    management_call::ManagementCallMetrics, provisioning::ProvisioningMetrics,
    system::SystemMetrics,
};

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

#[must_use]
pub fn placement_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("cascade", cascade_entries());
    entries.extend(prefix_entries("directory", directory_entries()));
    entries.extend(prefix_entries("pool", pool_entries()));
    entries.extend(prefix_entries("scaling", scaling_entries()));
    #[cfg(feature = "sharding")]
    entries.extend(prefix_entries("sharding", sharding_entries()));
    entries
}

#[must_use]
pub fn platform_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("platform_call", platform_call_entries());
    entries.extend(prefix_entries("http", http_entries()));
    entries.extend(prefix_entries(
        "inter_canister_call",
        inter_canister_call_entries(),
    ));
    entries
}

#[must_use]
pub fn runtime_entries() -> Vec<MetricEntry> {
    let mut entries = prefix_entries("intent", intent_entries());
    entries.extend(prefix_entries("perf", perf_entries()));
    entries.extend(prefix_entries("timer", timer_entries()));
    entries
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
    DirectoryMetrics::reset();
    HttpMetrics::reset();
    PlatformCallMetrics::reset();
    InterCanisterCallMetrics::reset();
    IntentMetrics::reset();
    LifecycleMetrics::reset();
    ManagementCallMetrics::reset();
    PoolMetrics::reset();
    ProvisioningMetrics::reset();
    ReplayMetrics::reset();
    RootCapabilityMetrics::reset();
    ScalingMetrics::reset();
    #[cfg(feature = "sharding")]
    ShardingMetrics::reset();
    SystemMetrics::reset();
    TimerMetrics::reset();
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

/// Project directory placement counters into the unified public metrics row shape.
#[must_use]
fn directory_entries() -> Vec<MetricEntry> {
    DirectoryMetrics::snapshot()
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

/// Project pool operation counters into the unified public metrics row shape.
#[must_use]
fn pool_entries() -> Vec<MetricEntry> {
    PoolMetrics::snapshot()
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
    CanisterOpsMetrics::snapshot()
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
    LifecycleMetrics::snapshot()
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

/// Project HTTP outcall counters into the unified public metrics row shape.
#[must_use]
fn http_entries() -> Vec<MetricEntry> {
    HttpMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![key.method.as_str().to_string(), key.label],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project timer counters into the unified public metrics row shape.
#[must_use]
fn timer_entries() -> Vec<MetricEntry> {
    TimerMetrics::snapshot()
        .entries
        .into_iter()
        .map(|(key, count)| MetricEntry {
            labels: vec![
                match key.mode {
                    TimerMode::Once => "once",
                    TimerMode::Interval => "interval",
                }
                .to_string(),
                key.label,
            ],
            principal: None,
            value: MetricValue::CountAndU64 {
                count,
                value_u64: key.delay_ms,
            },
        })
        .collect()
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
    CyclesFundingMetrics::snapshot()
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
    CyclesTopupMetrics::snapshot()
        .into_iter()
        .map(|(metric, count)| MetricEntry {
            labels: vec![metric.metric_label().to_string()],
            principal: None,
            value: MetricValue::Count(count),
        })
        .collect()
}

/// Project perf counters into the unified public metrics row shape.
#[must_use]
fn perf_entries() -> Vec<MetricEntry> {
    perf::entries()
        .into_iter()
        .map(|entry| {
            let labels = match entry.key {
                PerfKey::Endpoint(label) => vec!["endpoint".to_string(), label],
                PerfKey::Timer(label) => vec!["timer".to_string(), label],
                PerfKey::Checkpoint { scope, label } => {
                    vec!["checkpoint".to_string(), scope, label]
                }
            };

            MetricEntry {
                labels,
                principal: None,
                value: MetricValue::CountAndU64 {
                    count: entry.count,
                    value_u64: entry.total_instructions,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
