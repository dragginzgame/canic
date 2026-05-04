pub mod access;
pub mod auth;
pub mod canister_ops;
pub mod cascade;
pub mod cycles_funding;
pub mod cycles_topup;
pub mod delegated_auth;
pub mod directory;
pub mod http;
pub mod icc;
pub mod lifecycle;
pub mod pool;
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
    icc::IccMetrics,
    lifecycle::LifecycleMetrics,
    pool::PoolMetrics,
    replay::ReplayMetrics,
    root_capability::RootCapabilityMetrics,
    scaling::ScalingMetrics,
    system::SystemMetrics,
    timer::{TimerMetrics, TimerMode},
    wasm_store::WasmStoreMetrics,
};

#[cfg(feature = "sharding")]
use sharding::ShardingMetrics;

/// Project one metrics family into the unified public metrics row shape.
#[must_use]
pub fn entries(kind: MetricsKind) -> Vec<MetricEntry> {
    match kind {
        MetricsKind::Access => access_entries(),
        MetricsKind::Auth => auth_entries(),
        MetricsKind::CanisterOps => canister_ops_entries(),
        MetricsKind::Cascade => cascade_entries(),
        MetricsKind::CyclesFunding => cycles_funding_entries(),
        MetricsKind::CyclesTopup => cycles_topup_entries(),
        MetricsKind::DelegatedAuth => delegated_auth_entries(),
        MetricsKind::Directory => directory_entries(),
        MetricsKind::Http => http_entries(),
        MetricsKind::Icc => icc_entries(),
        MetricsKind::Lifecycle => lifecycle_entries(),
        MetricsKind::Perf => perf_entries(),
        MetricsKind::Pool => pool_entries(),
        MetricsKind::Replay => replay_entries(),
        MetricsKind::RootCapability => root_capability_entries(),
        MetricsKind::Scaling => scaling_entries(),
        #[cfg(feature = "sharding")]
        MetricsKind::Sharding => sharding_entries(),
        MetricsKind::System => system_entries(),
        MetricsKind::Timer => timer_entries(),
        MetricsKind::WasmStore => wasm_store_entries(),
    }
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
    IccMetrics::reset();
    LifecycleMetrics::reset();
    PoolMetrics::reset();
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

/// Project system counters into the unified public metrics row shape.
#[must_use]
fn system_entries() -> Vec<MetricEntry> {
    SystemMetrics::snapshot()
        .into_iter()
        .map(|(kind, count)| MetricEntry {
            labels: vec![
                match kind {
                    crate::ids::SystemMetricKind::CanisterCall => "CanisterCall",
                    crate::ids::SystemMetricKind::CanisterStatus => "CanisterStatus",
                    crate::ids::SystemMetricKind::CreateCanister => "CreateCanister",
                    crate::ids::SystemMetricKind::DeleteCanister => "DeleteCanister",
                    crate::ids::SystemMetricKind::DepositCycles => "DepositCycles",
                    crate::ids::SystemMetricKind::HttpOutcall => "HttpOutcall",
                    crate::ids::SystemMetricKind::InstallCode => "InstallCode",
                    crate::ids::SystemMetricKind::RawRand => "RawRand",
                    crate::ids::SystemMetricKind::ReinstallCode => "ReinstallCode",
                    crate::ids::SystemMetricKind::TimerScheduled => "TimerScheduled",
                    crate::ids::SystemMetricKind::UninstallCode => "UninstallCode",
                    crate::ids::SystemMetricKind::UpdateSettings => "UpdateSettings",
                    crate::ids::SystemMetricKind::UpgradeCode => "UpgradeCode",
                }
                .to_string(),
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
fn icc_entries() -> Vec<MetricEntry> {
    IccMetrics::snapshot()
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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "sharding")]
    use crate::ops::runtime::metrics::sharding::{
        ShardingMetricOperation, ShardingMetricOutcome, ShardingMetricReason, ShardingMetrics,
    };
    use crate::{
        cdk::types::Principal,
        ids::{AccessMetricKind, CanisterRole},
        ops::runtime::metrics::{
            auth::{
                AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface,
                AuthMetrics,
            },
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
                CanisterOpsMetrics,
            },
            cascade::{
                CascadeMetricOperation, CascadeMetricOutcome, CascadeMetricReason,
                CascadeMetricSnapshot, CascadeMetrics,
            },
            cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetrics},
            cycles_topup::CyclesTopupMetrics,
            delegated_auth::{
                DelegatedAuthMetricOperation, DelegatedAuthMetricOutcome, DelegatedAuthMetricReason,
            },
            directory::{
                DirectoryMetricOperation, DirectoryMetricOutcome, DirectoryMetricReason,
                DirectoryMetrics,
            },
            http::{HttpMethod, HttpMetrics},
            icc::IccMetrics,
            lifecycle::{
                LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole,
                LifecycleMetricStage, LifecycleMetrics,
            },
            pool::{PoolMetricOperation, PoolMetricOutcome, PoolMetricReason, PoolMetrics},
            replay::{
                ReplayMetricOperation, ReplayMetricOutcome, ReplayMetricReason, ReplayMetrics,
            },
            root_capability::{
                RootCapabilityMetricKey, RootCapabilityMetricOutcome,
                RootCapabilityMetricProofMode, RootCapabilityMetrics,
            },
            scaling::{
                ScalingMetricOperation, ScalingMetricOutcome, ScalingMetricReason, ScalingMetrics,
            },
            timer::{TimerMetrics, TimerMode},
            wasm_store::{
                WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
                WasmStoreMetricSource, WasmStoreMetrics,
            },
        },
    };
    use std::time::Duration;

    // Verify auth metrics expose stable label rows and accumulate counts.
    #[test]
    fn auth_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        AuthMetrics::record(
            AuthMetricSurface::Session,
            AuthMetricOperation::Bootstrap,
            AuthMetricOutcome::Rejected,
            AuthMetricReason::TokenInvalid,
        );
        AuthMetrics::record(
            AuthMetricSurface::Session,
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Created,
        );
        AuthMetrics::record(
            AuthMetricSurface::Session,
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Created,
        );
        AuthMetrics::record(
            AuthMetricSurface::Attestation,
            AuthMetricOperation::Verify,
            AuthMetricOutcome::Failed,
            AuthMetricReason::UnknownKeyId,
        );

        let entries = entries(MetricsKind::Auth);

        assert_metric_count(
            &entries,
            &["session", "bootstrap", "rejected", "token_invalid"],
            1,
        );
        assert_metric_count(&entries, &["session", "session", "completed", "created"], 2);
        assert_metric_count(
            &entries,
            &["attestation", "verify", "failed", "unknown_key_id"],
            1,
        );
    }

    // Verify canister operation metrics expose stable label rows and accumulate counts.
    #[test]
    fn canister_ops_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &CanisterRole::new("app"),
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Upgrade,
            &CanisterRole::new("worker"),
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::ManagementCall,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Upgrade,
            &CanisterRole::new("worker"),
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::ManagementCall,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &CanisterRole::new("worker"),
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::PoolReuse,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &CanisterRole::new("worker"),
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::StatePropagation,
        );

        let entries = entries(MetricsKind::CanisterOps);

        assert_metric_count(&entries, &["create", "app", "started", "ok"], 1);
        assert_metric_count(
            &entries,
            &["upgrade", "worker", "failed", "management_call"],
            2,
        );
        assert_metric_count(
            &entries,
            &["create", "worker", "completed", "pool_reuse"],
            1,
        );
        assert_metric_count(
            &entries,
            &["create", "worker", "failed", "state_propagation"],
            1,
        );
    }

    // Verify cascade metrics expose stable label rows and accumulate counts.
    #[test]
    fn cascade_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        CascadeMetrics::record(
            CascadeMetricOperation::RootFanout,
            CascadeMetricSnapshot::State,
            CascadeMetricOutcome::Started,
            CascadeMetricReason::Ok,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::ChildSend,
            CascadeMetricSnapshot::Topology,
            CascadeMetricOutcome::Failed,
            CascadeMetricReason::SendFailed,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::ChildSend,
            CascadeMetricSnapshot::Topology,
            CascadeMetricOutcome::Failed,
            CascadeMetricReason::SendFailed,
        );

        let entries = entries(MetricsKind::Cascade);

        assert_metric_count(&entries, &["root_fanout", "state", "started", "ok"], 1);
        assert_metric_count(
            &entries,
            &["child_send", "topology", "failed", "send_failed"],
            2,
        );
    }

    // Verify directory metrics expose stable label rows and accumulate counts.
    #[test]
    fn directory_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        DirectoryMetrics::record(
            DirectoryMetricOperation::Resolve,
            DirectoryMetricOutcome::Started,
            DirectoryMetricReason::Ok,
        );
        DirectoryMetrics::record(
            DirectoryMetricOperation::Classify,
            DirectoryMetricOutcome::Completed,
            DirectoryMetricReason::PendingFresh,
        );
        DirectoryMetrics::record(
            DirectoryMetricOperation::Classify,
            DirectoryMetricOutcome::Completed,
            DirectoryMetricReason::PendingFresh,
        );

        let entries = entries(MetricsKind::Directory);

        assert_metric_count(&entries, &["resolve", "started", "ok"], 1);
        assert_metric_count(&entries, &["classify", "completed", "pending_fresh"], 2);
    }

    // Verify wasm-store metrics expose stable label rows and accumulate counts.
    #[test]
    fn wasm_store_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        WasmStoreMetrics::record(
            WasmStoreMetricOperation::SourceResolve,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Store,
            WasmStoreMetricOutcome::Skipped,
            WasmStoreMetricReason::CacheHit,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Store,
            WasmStoreMetricOutcome::Skipped,
            WasmStoreMetricReason::CacheHit,
        );

        let entries = entries(MetricsKind::WasmStore);

        assert_metric_count(
            &entries,
            &["source_resolve", "bootstrap", "completed", "ok"],
            1,
        );
        assert_metric_count(
            &entries,
            &["chunk_upload", "store", "skipped", "cache_hit"],
            2,
        );
    }

    // Verify pool metrics expose stable label rows and accumulate counts.
    #[test]
    fn pool_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        PoolMetrics::record(
            PoolMetricOperation::Reset,
            PoolMetricOutcome::Started,
            PoolMetricReason::Ok,
        );
        PoolMetrics::record(
            PoolMetricOperation::ImportQueued,
            PoolMetricOutcome::Skipped,
            PoolMetricReason::AlreadyPresent,
        );
        PoolMetrics::record(
            PoolMetricOperation::ImportQueued,
            PoolMetricOutcome::Skipped,
            PoolMetricReason::AlreadyPresent,
        );

        let entries = entries(MetricsKind::Pool);

        assert_metric_count(&entries, &["reset", "started", "ok"], 1);
        assert_metric_count(
            &entries,
            &["import_queued", "skipped", "already_present"],
            2,
        );
    }

    // Verify scaling metrics expose stable label rows and accumulate counts.
    #[test]
    fn scaling_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        ScalingMetrics::record(
            ScalingMetricOperation::PlanCreate,
            ScalingMetricOutcome::Completed,
            ScalingMetricReason::BelowMinWorkers,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::BootstrapPool,
            ScalingMetricOutcome::Skipped,
            ScalingMetricReason::TargetSatisfied,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::BootstrapPool,
            ScalingMetricOutcome::Skipped,
            ScalingMetricReason::TargetSatisfied,
        );

        let entries = entries(MetricsKind::Scaling);

        assert_metric_count(
            &entries,
            &["plan_create", "completed", "below_min_workers"],
            1,
        );
        assert_metric_count(
            &entries,
            &["bootstrap_pool", "skipped", "target_satisfied"],
            2,
        );
    }

    // Verify sharding metrics expose stable label rows and accumulate counts.
    #[cfg(feature = "sharding")]
    #[test]
    fn sharding_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        ShardingMetrics::record(
            ShardingMetricOperation::PlanAssign,
            ShardingMetricOutcome::Completed,
            ShardingMetricReason::ExistingCapacity,
        );
        ShardingMetrics::record(
            ShardingMetricOperation::BootstrapPool,
            ShardingMetricOutcome::Skipped,
            ShardingMetricReason::TargetSatisfied,
        );
        ShardingMetrics::record(
            ShardingMetricOperation::BootstrapPool,
            ShardingMetricOutcome::Skipped,
            ShardingMetricReason::TargetSatisfied,
        );

        let entries = entries(MetricsKind::Sharding);

        assert_metric_count(
            &entries,
            &["plan_assign", "completed", "existing_capacity"],
            1,
        );
        assert_metric_count(
            &entries,
            &["bootstrap_pool", "skipped", "target_satisfied"],
            2,
        );
    }

    // Verify lifecycle metrics expose stable label rows and accumulate counts.
    #[test]
    fn lifecycle_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        LifecycleMetrics::record(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricStage::Runtime,
            LifecycleMetricOutcome::Started,
        );
        LifecycleMetrics::record(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricStage::Runtime,
            LifecycleMetricOutcome::Started,
        );
        LifecycleMetrics::record(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricStage::Bootstrap,
            LifecycleMetricOutcome::Completed,
        );

        let entries = entries(MetricsKind::Lifecycle);

        assert_metric_count(&entries, &["init", "root", "runtime", "started"], 2);
        assert_metric_count(
            &entries,
            &["post_upgrade", "nonroot", "bootstrap", "completed"],
            1,
        );
    }

    #[test]
    fn cycles_topup_metrics_are_exposed() {
        reset_for_tests();

        CyclesTopupMetrics::record_policy_missing();
        CyclesTopupMetrics::record_request_scheduled();
        CyclesTopupMetrics::record_request_scheduled();

        let entries = entries(MetricsKind::CyclesTopup);

        assert_metric_count(&entries, &["policy_missing"], 1);
        assert_metric_count(&entries, &["request_scheduled"], 2);
    }

    // Verify replay metrics expose stable label rows and accumulate counts.
    #[test]
    fn replay_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Completed,
            ReplayMetricReason::Fresh,
        );
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Failed,
            ReplayMetricReason::Conflict,
        );
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Failed,
            ReplayMetricReason::Conflict,
        );

        let entries = entries(MetricsKind::Replay);

        assert_metric_count(&entries, &["check", "completed", "fresh"], 1);
        assert_metric_count(&entries, &["check", "failed", "conflict"], 2);
    }

    // Verify delegated-auth metrics expose authority and outcome rows.
    #[test]
    fn delegated_auth_metrics_are_exposed_with_stable_labels() {
        reset_for_tests();

        let principal = Principal::from_slice(&[42; 29]);
        DelegatedAuthMetrics::record_authority(principal);
        DelegatedAuthMetrics::record_verify_started();
        DelegatedAuthMetrics::record_verify_completed();
        DelegatedAuthMetrics::record(
            DelegatedAuthMetricOperation::VerifyToken,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::TokenExpired,
        );
        DelegatedAuthMetrics::record(
            DelegatedAuthMetricOperation::VerifyToken,
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::TokenExpired,
        );

        let entries = entries(MetricsKind::DelegatedAuth);

        assert_metric_count(&entries, &["delegated_auth_authority"], 1);
        assert_metric_count(&entries, &["verify_token", "started", "ok"], 1);
        assert_metric_count(&entries, &["verify_token", "completed", "ok"], 1);
        assert_metric_count(&entries, &["verify_token", "failed", "token_expired"], 2);
    }

    #[test]
    fn reset_for_tests_clears_all_metric_families() {
        reset_for_tests();
        let principal = Principal::from_slice(&[42; 29]);

        AccessMetrics::increment("create_project", AccessMetricKind::Guard, "controller_only");
        AuthMetrics::record(
            AuthMetricSurface::Session,
            AuthMetricOperation::Session,
            AuthMetricOutcome::Completed,
            AuthMetricReason::Created,
        );
        CanisterOpsMetrics::record(
            CanisterOpsMetricOperation::Create,
            &CanisterRole::new("app"),
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );
        CascadeMetrics::record(
            CascadeMetricOperation::RootFanout,
            CascadeMetricSnapshot::State,
            CascadeMetricOutcome::Started,
            CascadeMetricReason::Ok,
        );
        CyclesFundingMetrics::record_denied(
            principal,
            10,
            CyclesFundingDeniedReason::ChildNotFound,
        );
        CyclesTopupMetrics::record_request_scheduled();
        DelegatedAuthMetrics::record_authority(principal);
        DirectoryMetrics::record(
            DirectoryMetricOperation::Resolve,
            DirectoryMetricOutcome::Started,
            DirectoryMetricReason::Ok,
        );
        HttpMetrics::record_http_request(HttpMethod::Get, "https://example.test/a", Some("api"));
        IccMetrics::record_call(principal, "canic_sync");
        LifecycleMetrics::record(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Nonroot,
            LifecycleMetricStage::Bootstrap,
            LifecycleMetricOutcome::Started,
        );
        PoolMetrics::record(
            PoolMetricOperation::Reset,
            PoolMetricOutcome::Started,
            PoolMetricReason::Ok,
        );
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Completed,
            ReplayMetricReason::Fresh,
        );
        RootCapabilityMetrics::record_proof(
            RootCapabilityMetricKey::Provision,
            RootCapabilityMetricOutcome::Accepted,
            RootCapabilityMetricProofMode::Structural,
        );
        ScalingMetrics::record(
            ScalingMetricOperation::PlanCreate,
            ScalingMetricOutcome::Started,
            ScalingMetricReason::Ok,
        );
        #[cfg(feature = "sharding")]
        ShardingMetrics::record(
            ShardingMetricOperation::PlanAssign,
            ShardingMetricOutcome::Started,
            ShardingMetricReason::Ok,
        );
        TimerMetrics::record_timer_scheduled(TimerMode::Once, Duration::from_secs(1), "once:test");
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::SourceResolve,
            WasmStoreMetricSource::Embedded,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        perf::record_checkpoint("metrics::tests", "checkpoint", 7);

        for kind in all_metric_kinds() {
            assert!(!entries(*kind).is_empty());
        }

        reset_for_tests();

        for kind in all_metric_kinds() {
            assert!(entries(*kind).is_empty());
        }
    }

    // Return every public metric family for reset coverage.
    fn all_metric_kinds() -> &'static [MetricsKind] {
        &[
            MetricsKind::Access,
            MetricsKind::Auth,
            MetricsKind::CanisterOps,
            MetricsKind::Cascade,
            MetricsKind::CyclesFunding,
            MetricsKind::CyclesTopup,
            MetricsKind::DelegatedAuth,
            MetricsKind::Directory,
            MetricsKind::Http,
            MetricsKind::Icc,
            MetricsKind::Lifecycle,
            MetricsKind::Perf,
            MetricsKind::Pool,
            MetricsKind::Replay,
            MetricsKind::RootCapability,
            MetricsKind::Scaling,
            #[cfg(feature = "sharding")]
            MetricsKind::Sharding,
            MetricsKind::System,
            MetricsKind::Timer,
            MetricsKind::WasmStore,
        ]
    }

    fn assert_metric_count(entries: &[MetricEntry], labels: &[&str], expected: u64) {
        let entry = entries
            .iter()
            .find(|entry| entry.labels.iter().map(String::as_str).collect::<Vec<_>>() == labels)
            .expect("metric entry should exist");

        match &entry.value {
            MetricValue::Count(count) => assert_eq!(*count, expected),
            _ => panic!("metric entry should use Count"),
        }
    }
}
