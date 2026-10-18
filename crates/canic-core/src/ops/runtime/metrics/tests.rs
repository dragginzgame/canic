//! Module: ops::runtime::metrics::tests
//!
//! Responsibility: validate metrics projection and reset behavior across metric families.
//! Does not own: production metrics recording or workflow decisions.
//! Boundary: test-only coverage for ops-layer metrics projection.

use super::*;

#[cfg(feature = "sharding")]
use crate::ops::runtime::metrics::sharding::{
    ShardingMetricOperation, ShardingMetricOutcome, ShardingMetricReason, ShardingMetrics,
};
use crate::{
    cdk::types::Principal,
    ops::{
        runtime::metrics::{
            auth::{AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface},
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            },
            cascade::{CascadeMetricOperation, CascadeMetricOutcome, CascadeMetricReason},
            cycles_funding::CyclesFundingDeniedReason,
            icp_refill::entries_from_snapshot,
            intent::{
                IntentMetricOperation, IntentMetricOutcome, IntentMetricReason, IntentMetricSurface,
            },
            lifecycle::{
                LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole,
                LifecycleMetricStage,
            },
            placement_index::{
                PlacementIndexMetricOperation, PlacementIndexMetricOutcome,
                PlacementIndexMetricReason,
            },
            platform_call::{
                PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
                PlatformCallMetricSurface,
            },
            replay::{ReplayMetricOperation, ReplayMetricOutcome, ReplayMetricReason},
            root_capability::{
                RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetricProofMode,
            },
            scaling::{ScalingMetricOperation, ScalingMetricOutcome, ScalingMetricReason},
            wasm_store::{
                WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
                WasmStoreMetricSource,
            },
        },
        storage::icp_refill::{
            IcpRefillMetricErrorCount, IcpRefillMetricSnapshot, IcpRefillMetricStatusCount,
            IcpRefillMetricTargetTotal,
        },
    },
    storage::stable::icp_refill::{IcpRefillRecordErrorCode, IcpRefillRecordStatus},
};
use canic_contracts::ids::{
    AccessMetricKind, CanisterRole, EndpointCall, EndpointCallKind, EndpointId,
};

fn endpoint_call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
    EndpointCall {
        endpoint: EndpointId::new(name),
        kind,
    }
}

#[test]
fn perf_projection_preserves_wide_totals_empty_zero_and_independent_quality() {
    use ic_metrics::MeasurementAggregate as Aggregate;
    let aggregates = [
        Aggregate::EMPTY,
        Aggregate::from_parts(1, 0, Some(0), false, false).unwrap(),
        Aggregate::from_parts(2, u128::from(u64::MAX), Some(u64::MAX), false, false).unwrap(),
        Aggregate::from_parts(2, u128::from(u64::MAX) + 1, Some(u64::MAX), false, false).unwrap(),
        Aggregate::from_parts(u64::MAX, 0, Some(0), true, false).unwrap(),
        Aggregate::from_parts(u64::MAX, u128::MAX, Some(u64::MAX), true, true).unwrap(),
    ];
    let snapshot = aggregates
        .iter()
        .enumerate()
        .map(|(index, aggregate)| perf::PerfEntry {
            key: PerfKey::Checkpoint {
                scope: "wide-total".into(),
                label: index.to_string(),
            },
            aggregate: *aggregate,
        })
        .collect();
    let entries = perf_entries_from(snapshot, &[]);
    assert_eq!(entries.len(), aggregates.len());
    for (entry, source) in entries.iter().zip(aggregates) {
        let MetricValue::Measurement {
            count,
            total,
            count_overflowed,
            total_overflowed,
        } = entry.value
        else {
            panic!("performance must preserve the measurement payload")
        };
        assert_eq!(count, source.samples());
        assert_eq!(total, source.total());
        assert_eq!(count_overflowed, source.samples_overflowed());
        assert_eq!(total_overflowed, source.total_overflowed());
    }
}

#[test]
fn timer_diagnostics_keep_starts_and_latest_delay_independent() {
    for (delay, expected_rows) in [(None, 1), (Some(0), 2), (Some(500_000_000), 2)] {
        let rows = timer_rows(
            vec!["once".into(), "app".into(), "jobs".into(), "tick".into()],
            4,
            delay,
        );
        assert_eq!(rows.len(), expected_rows);
        assert_eq!(
            rows[0].labels,
            ["once", "app", "jobs", "tick", "work_started"]
        );
        assert!(matches!(
            rows[0].value,
            MetricValue::Count {
                count: 4,
                exact: true
            }
        ));
        if let Some(delay) = delay {
            assert_eq!(
                rows[1].labels,
                ["once", "app", "jobs", "tick", "latest_armed_delay_ms"]
            );
            assert!(
                matches!(rows[1].value, MetricValue::U128 { value, .. } if value == u128::from(delay / 1_000_000))
            );
        }
    }
}

#[test]
fn auth_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Cleanup,
        AuthMetricOutcome::Failed,
        AuthMetricReason::VerifyFailed,
    );
    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Cleanup,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Expired,
    );
    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Cleanup,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Expired,
    );
    AuthMetrics::record(
        AuthMetricSurface::Attestation,
        AuthMetricOperation::Verify,
        AuthMetricOutcome::Failed,
        AuthMetricReason::VerifyFailed,
    );

    let entries = entries(MetricsKind::Security);

    assert_metric_count(
        &entries,
        &[
            "auth",
            "application_session",
            "cleanup",
            "failed",
            "verify_failed",
        ],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "auth",
            "application_session",
            "cleanup",
            "completed",
            "expired",
        ],
        2,
    );
    assert_metric_count(
        &entries,
        &["auth", "attestation", "verify", "failed", "verify_failed"],
        1,
    );
}

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
        CanisterOpsMetricOperation::Reinstall,
        &CanisterRole::new("worker"),
        CanisterOpsMetricOutcome::Failed,
        CanisterOpsMetricReason::ManagementCall,
    );
    CanisterOpsMetrics::record(
        CanisterOpsMetricOperation::Reinstall,
        &CanisterRole::new("worker"),
        CanisterOpsMetricOutcome::Failed,
        CanisterOpsMetricReason::ManagementCall,
    );
    CanisterOpsMetrics::record(
        CanisterOpsMetricOperation::Create,
        &CanisterRole::new("worker"),
        CanisterOpsMetricOutcome::Failed,
        CanisterOpsMetricReason::Topology,
    );

    let entries = entries(MetricsKind::Core);

    assert_metric_count(
        &entries,
        &["canister_ops", "create", "app", "started", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "canister_ops",
            "reinstall",
            "worker",
            "failed",
            "management_call",
        ],
        2,
    );
    assert_metric_count(
        &entries,
        &["canister_ops", "create", "worker", "failed", "topology"],
        1,
    );
}

#[test]
fn perf_endpoint_metrics_include_call_kind_label() {
    reset_for_tests();

    perf::record_endpoint_call(endpoint_call("read_state", EndpointCallKind::Query), 10);
    perf::record_endpoint_call(
        endpoint_call("read_remote_state", EndpointCallKind::QueryComposite),
        20,
    );
    perf::record_endpoint_call(endpoint_call("write_state", EndpointCallKind::Update), 30);

    let entries = entries(MetricsKind::Runtime);

    assert_metric_measurement(
        &entries,
        &["perf", "endpoint", "query", "read_state"],
        1,
        10,
    );
    assert_metric_measurement(
        &entries,
        &["perf", "endpoint", "composite_query", "read_remote_state"],
        1,
        20,
    );
    assert_metric_measurement(
        &entries,
        &["perf", "endpoint", "update", "write_state"],
        1,
        30,
    );
}

#[test]
fn timer_inventory_availability_distinguishes_unavailable_from_empty() {
    let available = timer_inventory_availability(true);
    let unavailable = timer_inventory_availability(false);

    assert_eq!(available.labels, ["inventory", "available"]);
    assert!(matches!(
        available.value,
        MetricValue::Count {
            count: 1,
            exact: true
        }
    ));
    assert_eq!(unavailable.labels, ["inventory", "available"]);
    assert!(matches!(
        unavailable.value,
        MetricValue::Count {
            count: 0,
            exact: true
        }
    ));
}

#[test]
fn cascade_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    CascadeMetrics::record(
        CascadeMetricOperation::RootFanout,
        crate::ops::runtime::metrics::cascade::CascadeMetricSnapshot::State,
        CascadeMetricOutcome::Started,
        CascadeMetricReason::Ok,
    );
    CascadeMetrics::record(
        CascadeMetricOperation::ChildSend,
        crate::ops::runtime::metrics::cascade::CascadeMetricSnapshot::Topology,
        CascadeMetricOutcome::Failed,
        CascadeMetricReason::SendFailed,
    );
    CascadeMetrics::record(
        CascadeMetricOperation::ChildSend,
        crate::ops::runtime::metrics::cascade::CascadeMetricSnapshot::Topology,
        CascadeMetricOutcome::Failed,
        CascadeMetricReason::SendFailed,
    );

    let entries = entries(MetricsKind::Placement);

    assert_metric_count(
        &entries,
        &["cascade", "root_fanout", "state", "started", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["cascade", "child_send", "topology", "failed", "send_failed"],
        2,
    );
}

#[test]
fn placement_index_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    PlacementIndexMetrics::record(
        PlacementIndexMetricOperation::Resolve,
        PlacementIndexMetricOutcome::Started,
        PlacementIndexMetricReason::Ok,
    );
    PlacementIndexMetrics::record(
        PlacementIndexMetricOperation::Classify,
        PlacementIndexMetricOutcome::Completed,
        PlacementIndexMetricReason::PendingFresh,
    );
    PlacementIndexMetrics::record(
        PlacementIndexMetricOperation::Classify,
        PlacementIndexMetricOutcome::Completed,
        PlacementIndexMetricReason::PendingFresh,
    );

    let entries = entries(MetricsKind::Placement);

    assert_metric_count(
        &entries,
        &["placement_index", "resolve", "started", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["placement_index", "classify", "completed", "pending_fresh"],
        2,
    );
}

#[test]
fn wasm_store_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    WasmStoreMetrics::record(
        WasmStoreMetricOperation::ReleasePublish,
        WasmStoreMetricSource::TargetStore,
        WasmStoreMetricOutcome::Completed,
        WasmStoreMetricReason::Ok,
    );
    WasmStoreMetrics::record(
        WasmStoreMetricOperation::ChunkUpload,
        WasmStoreMetricSource::TargetStore,
        WasmStoreMetricOutcome::Skipped,
        WasmStoreMetricReason::CacheHit,
    );
    WasmStoreMetrics::record(
        WasmStoreMetricOperation::ChunkUpload,
        WasmStoreMetricSource::TargetStore,
        WasmStoreMetricOutcome::Skipped,
        WasmStoreMetricReason::CacheHit,
    );

    let entries = entries(MetricsKind::Storage);

    assert_metric_count(
        &entries,
        &[
            "wasm_store",
            "release_publish",
            "target_store",
            "completed",
            "ok",
        ],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "wasm_store",
            "chunk_upload",
            "target_store",
            "skipped",
            "cache_hit",
        ],
        2,
    );
}

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

    let entries = entries(MetricsKind::Placement);

    assert_metric_count(
        &entries,
        &["scaling", "plan_create", "completed", "below_min_workers"],
        1,
    );
    assert_metric_count(
        &entries,
        &["scaling", "bootstrap_pool", "skipped", "target_satisfied"],
        2,
    );
}

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
    ShardingMetrics::record(
        ShardingMetricOperation::ReleaseKey,
        ShardingMetricOutcome::Skipped,
        ShardingMetricReason::NotAssigned,
    );

    let entries = entries(MetricsKind::Placement);

    assert_metric_count(
        &entries,
        &["sharding", "plan_assign", "completed", "existing_capacity"],
        1,
    );
    assert_metric_count(
        &entries,
        &["sharding", "bootstrap_pool", "skipped", "target_satisfied"],
        2,
    );
    assert_metric_count(
        &entries,
        &["sharding", "release_key", "skipped", "not_assigned"],
        1,
    );
}

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

    let entries = entries(MetricsKind::Core);

    assert_metric_count(
        &entries,
        &["lifecycle", "init", "root", "runtime", "started"],
        2,
    );
    assert_metric_count(
        &entries,
        &[
            "lifecycle",
            "post_upgrade",
            "nonroot",
            "bootstrap",
            "completed",
        ],
        1,
    );
}

#[test]
fn cycles_topup_metrics_are_exposed() {
    reset_for_tests();

    CyclesTopupMetrics::record_policy_missing();
    CyclesTopupMetrics::record_request_scheduled();
    CyclesTopupMetrics::record_request_scheduled();

    let entries = entries(MetricsKind::Core);

    assert_metric_count(&entries, &["cycles_topup", "policy_missing"], 1);
    assert_metric_count(&entries, &["cycles_topup", "request_scheduled"], 2);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the metric projection journey keeps one record-state assertion matrix together"
)]
fn icp_refill_metrics_project_bounded_record_state() {
    let target = Principal::from_slice(&[22; 29]);
    let other_target = Principal::from_slice(&[23; 29]);
    let entries = prefix_entries(
        "cycles_funding",
        entries_from_snapshot(&IcpRefillMetricSnapshot {
            statuses: vec![
                IcpRefillMetricStatusCount {
                    trigger_label: "automatic",
                    phase: "notify",
                    status: IcpRefillRecordStatus::Completed,
                    count: 1,
                },
                IcpRefillMetricStatusCount {
                    trigger_label: "manual",
                    phase: "notify",
                    status: IcpRefillRecordStatus::Failed,
                    count: 2,
                },
            ],
            errors: vec![IcpRefillMetricErrorCount {
                error_code: IcpRefillRecordErrorCode::NotifyFailed,
                count: 2,
            }],
            targets: vec![
                IcpRefillMetricTargetTotal {
                    target_canister: target,
                    amount_e8s: 300,
                    cycles_sent: Some(4_000),
                },
                IcpRefillMetricTargetTotal {
                    target_canister: other_target,
                    amount_e8s: 300,
                    cycles_sent: None,
                },
            ],
        }),
    );

    assert_metric_count(
        &entries,
        &[
            "cycles_funding",
            "icp_refill",
            "automatic",
            "notify",
            "status",
            "completed",
        ],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "cycles_funding",
            "icp_refill",
            "manual",
            "notify",
            "status",
            "failed",
        ],
        2,
    );
    assert_metric_count(
        &entries,
        &[
            "cycles_funding",
            "icp_refill",
            "notify",
            "error",
            "notify_failed",
        ],
        2,
    );
    assert_metric_u128(
        &entries,
        &[
            "cycles_funding",
            "icp_refill",
            "transfer",
            "amount_e8s",
            "target",
        ],
        Some(target),
        300,
    );
    assert_metric_u128(
        &entries,
        &[
            "cycles_funding",
            "icp_refill",
            "transfer",
            "amount_e8s",
            "target",
        ],
        Some(other_target),
        300,
    );
    assert_metric_u128(
        &entries,
        &[
            "cycles_funding",
            "icp_refill",
            "notify",
            "cycles_sent",
            "target",
        ],
        Some(target),
        4_000,
    );
}

#[test]
fn platform_call_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Generic,
        PlatformCallMetricMode::BoundedWait,
        PlatformCallMetricOutcome::Started,
        PlatformCallMetricReason::Ok,
    );
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Management,
        PlatformCallMetricMode::Update,
        PlatformCallMetricOutcome::Failed,
        PlatformCallMetricReason::Infra,
    );
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Management,
        PlatformCallMetricMode::Update,
        PlatformCallMetricOutcome::Failed,
        PlatformCallMetricReason::Infra,
    );

    let entries = entries(MetricsKind::Platform);

    assert_metric_count(
        &entries,
        &["platform_call", "generic", "bounded_wait", "started", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["platform_call", "management", "update", "failed", "infra"],
        2,
    );
}

#[test]
fn intent_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    IntentMetrics::record(
        IntentMetricSurface::Local,
        IntentMetricOperation::Reserve,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    IntentMetrics::record(
        IntentMetricSurface::Local,
        IntentMetricOperation::Commit,
        IntentMetricOutcome::Failed,
        IntentMetricReason::StorageFailed,
    );
    IntentMetrics::record(
        IntentMetricSurface::ReceiptBacked,
        IntentMetricOperation::Abort,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    IntentMetrics::record(
        IntentMetricSurface::Local,
        IntentMetricOperation::Commit,
        IntentMetricOutcome::Failed,
        IntentMetricReason::StorageFailed,
    );

    let entries = entries(MetricsKind::Runtime);

    assert_metric_count(
        &entries,
        &["intent", "local", "reserve", "completed", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["intent", "local", "commit", "failed", "storage_failed"],
        2,
    );
    assert_metric_count(
        &entries,
        &["intent", "receipt_backed", "abort", "completed", "ok"],
        1,
    );
}

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

    let entries = entries(MetricsKind::Security);

    assert_metric_count(&entries, &["replay", "check", "completed", "fresh"], 1);
    assert_metric_count(&entries, &["replay", "check", "failed", "conflict"], 2);
}

#[test]
fn delegated_auth_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();
    DelegatedAuthMetrics::record_authority(Principal::from_slice(&[42; 29]));
    DelegatedAuthMetrics::record_root_proof_prepare_started();
    DelegatedAuthMetrics::record_root_proof_prepare_completed();
    DelegatedAuthMetrics::record_root_proof_prepare_failed();
    DelegatedAuthMetrics::record_root_proof_prepare_failed();
    DelegatedAuthMetrics::record_renewal_sweep_completed();
    let entries = entries(MetricsKind::Security);
    assert_metric_count(&entries, &["delegated_auth", "delegated_auth_authority"], 1);
    assert_metric_count(
        &entries,
        &["delegated_auth", "prepare_root_proof", "completed", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "delegated_auth",
            "prepare_root_proof",
            "failed",
            "root_proof_prepare_failed",
        ],
        2,
    );
    assert_metric_count(
        &entries,
        &["delegated_auth", "renewal_sweep", "completed", "ok"],
        1,
    );
}

#[test]
fn reset_for_tests_clears_all_metric_families() {
    reset_for_tests();
    seed_all_metric_families_for_reset_test();

    for kind in all_metric_kinds() {
        assert!(!entries(*kind).is_empty());
    }

    reset_for_tests();

    for kind in all_metric_kinds() {
        let entries = entries(*kind);
        if matches!(kind, MetricsKind::Runtime) {
            assert_eq!(entries.len(), 1);
            assert_metric_count(&entries, &["timer", "inventory", "available"], 0);
        } else {
            assert!(entries.is_empty());
        }
    }
}

fn seed_all_metric_families_for_reset_test() {
    let principal = Principal::from_slice(&[42; 29]);

    AccessMetrics::increment("create_project", AccessMetricKind::Guard, "controller_only");
    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Cleanup,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Expired,
    );
    CanisterOpsMetrics::record(
        CanisterOpsMetricOperation::Create,
        &CanisterRole::new("app"),
        CanisterOpsMetricOutcome::Started,
        CanisterOpsMetricReason::Ok,
    );
    CascadeMetrics::record(
        CascadeMetricOperation::RootFanout,
        crate::ops::runtime::metrics::cascade::CascadeMetricSnapshot::State,
        CascadeMetricOutcome::Started,
        CascadeMetricReason::Ok,
    );
    CyclesFundingMetrics::record_denied(principal, 10, CyclesFundingDeniedReason::ChildNotFound);
    CyclesTopupMetrics::record_request_scheduled();
    DelegatedAuthMetrics::record_authority(principal);
    PlacementIndexMetrics::record(
        PlacementIndexMetricOperation::Resolve,
        PlacementIndexMetricOutcome::Started,
        PlacementIndexMetricReason::Ok,
    );
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Generic,
        PlatformCallMetricMode::BoundedWait,
        PlatformCallMetricOutcome::Started,
        PlatformCallMetricReason::Ok,
    );
    InterCanisterCallMetrics::record_call(principal, "canic_sync");
    IntentMetrics::record(
        IntentMetricSurface::Local,
        IntentMetricOperation::Reserve,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    LifecycleMetrics::record(
        LifecycleMetricPhase::Init,
        LifecycleMetricRole::Nonroot,
        LifecycleMetricStage::Bootstrap,
        LifecycleMetricOutcome::Started,
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
    WasmStoreMetrics::record(
        WasmStoreMetricOperation::ReleasePublish,
        WasmStoreMetricSource::ManagedFleet,
        WasmStoreMetricOutcome::Completed,
        WasmStoreMetricReason::Ok,
    );
    perf::record_checkpoint("metrics::tests", "checkpoint", 7);
}

fn all_metric_kinds() -> &'static [MetricsKind] {
    &[
        MetricsKind::Core,
        MetricsKind::Placement,
        MetricsKind::Platform,
        MetricsKind::Runtime,
        MetricsKind::Security,
        MetricsKind::Storage,
    ]
}

fn assert_metric_count(entries: &[MetricEntry], labels: &[&str], expected: u128) {
    let entry = entries
        .iter()
        .find(|entry| entry.labels.iter().map(String::as_str).collect::<Vec<_>>() == labels)
        .expect("metric entry should exist");

    match &entry.value {
        MetricValue::Count { count, .. } => assert_eq!(*count, expected),
        _ => panic!("metric entry should use Count"),
    }
}

fn assert_metric_measurement(
    entries: &[MetricEntry],
    labels: &[&str],
    expected_count: u64,
    expected_value: u128,
) {
    let entry = entries
        .iter()
        .find(|entry| entry.labels.iter().map(String::as_str).collect::<Vec<_>>() == labels)
        .expect("metric entry should exist");

    match &entry.value {
        MetricValue::Measurement {
            count,
            total,
            count_overflowed,
            total_overflowed,
        } => {
            assert_eq!(*count, expected_count);
            assert_eq!(*total, expected_value);
            assert!(!count_overflowed && !total_overflowed);
        }
        _ => panic!("metric entry should retain measurement quality"),
    }
}

fn assert_metric_u128(
    entries: &[MetricEntry],
    labels: &[&str],
    principal: Option<Principal>,
    expected: u128,
) {
    let entry = entries
        .iter()
        .find(|entry| {
            entry.labels.iter().map(String::as_str).collect::<Vec<_>>() == labels
                && entry.principal == principal
        })
        .expect("metric entry should exist");

    match &entry.value {
        MetricValue::U128 { value, .. } => assert_eq!(*value, expected),
        _ => panic!("metric entry should use U128"),
    }
}

#[test]
fn scalar_count_projection_preserves_wide_totals_and_source_quality() {
    for (total, overflowed) in [
        (0, false),
        (u128::from(u64::MAX), false),
        (u128::from(u64::MAX) + 1, false),
        (u128::MAX, false),
        (u128::MAX, true),
    ] {
        let source = ic_metrics::MeasurementCounter::from_parts(total, overflowed).unwrap();
        assert!(
            matches!(count_value(source), MetricValue::Count { count, exact }
            if count == total && exact != overflowed)
        );
    }
    let forwarded = timer_rows(vec!["test".into()], u64::MAX, Some(0));
    assert!(
        matches!(forwarded[0].value, MetricValue::Count { count, exact: false }
        if count == u128::from(u64::MAX))
    );
    assert!(matches!(
        forwarded[1].value,
        MetricValue::U128 {
            value: 0,
            exact: true
        }
    ));
}
