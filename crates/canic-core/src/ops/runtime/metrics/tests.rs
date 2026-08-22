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
    ids::{AccessMetricKind, CanisterRole, EndpointCall, EndpointCallKind, EndpointId},
    ops::{
        runtime::metrics::{
            auth::{AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface},
            canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            },
            cascade::{CascadeMetricOperation, CascadeMetricOutcome, CascadeMetricReason},
            cycles_funding::CyclesFundingDeniedReason,
            delegated_auth::{
                DelegatedAuthMetricOperation, DelegatedAuthMetricOutcome, DelegatedAuthMetricReason,
            },
            icp_refill::entries_from_snapshot,
            intent::{
                IntentMetricOperation, IntentMetricOutcome, IntentMetricReason, IntentMetricSurface,
            },
            lifecycle::{
                LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole,
                LifecycleMetricStage,
            },
            management_call::{
                ManagementCallMetricOperation, ManagementCallMetricOutcome,
                ManagementCallMetricReason,
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

fn endpoint_call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
    EndpointCall {
        endpoint: EndpointId::new(name),
        kind,
    }
}

#[test]
fn auth_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Rejected,
        AuthMetricReason::ProofInvalid,
    );
    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Created,
    );
    AuthMetrics::record(
        AuthMetricSurface::ApplicationSession,
        AuthMetricOperation::Establish,
        AuthMetricOutcome::Completed,
        AuthMetricReason::Created,
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
            "establish",
            "rejected",
            "proof_invalid",
        ],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "auth",
            "application_session",
            "establish",
            "completed",
            "created",
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

    assert_metric_count_and_u64(
        &entries,
        &["perf", "endpoint", "query", "read_state"],
        1,
        10,
    );
    assert_metric_count_and_u64(
        &entries,
        &["perf", "endpoint", "composite_query", "read_remote_state"],
        1,
        20,
    );
    assert_metric_count_and_u64(
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
    assert!(matches!(available.value, MetricValue::Count(1)));
    assert_eq!(unavailable.labels, ["inventory", "available"]);
    assert!(matches!(unavailable.value, MetricValue::Count(0)));
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

    let entries = entries(MetricsKind::Storage);

    assert_metric_count(
        &entries,
        &[
            "wasm_store",
            "source_resolve",
            "bootstrap",
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
            "store",
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
fn management_call_metrics_remain_internal_platform_counters() {
    reset_for_tests();

    ManagementCallMetrics::record(
        ManagementCallMetricOperation::InstallCode,
        ManagementCallMetricOutcome::Started,
        ManagementCallMetricReason::Ok,
    );
    ManagementCallMetrics::record(
        ManagementCallMetricOperation::InstallCode,
        ManagementCallMetricOutcome::Failed,
        ManagementCallMetricReason::Infra,
    );
    ManagementCallMetrics::record(
        ManagementCallMetricOperation::InstallCode,
        ManagementCallMetricOutcome::Failed,
        ManagementCallMetricReason::Infra,
    );

    let snapshot = ManagementCallMetrics::snapshot();
    assert_eq!(snapshot.len(), 2);
    assert!(snapshot.iter().any(|(key, count)| key.operation
        == ManagementCallMetricOperation::InstallCode
        && key.outcome == ManagementCallMetricOutcome::Started
        && key.reason == ManagementCallMetricReason::Ok
        && *count == 1));
    assert!(snapshot.iter().any(|(key, count)| key.operation
        == ManagementCallMetricOperation::InstallCode
        && key.outcome == ManagementCallMetricOutcome::Failed
        && key.reason == ManagementCallMetricReason::Infra
        && *count == 2));
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
                    trigger: crate::domain::icp_refill::IcpRefillTrigger::Automatic { sequence: 1 },
                    status: IcpRefillRecordStatus::Completed,
                    error_code: None,
                    count: 1,
                },
                IcpRefillMetricStatusCount {
                    trigger: crate::domain::icp_refill::IcpRefillTrigger::Manual,
                    status: IcpRefillRecordStatus::Failed,
                    error_code: Some(IcpRefillRecordErrorCode::NotifyFailed),
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

    let principal = Principal::from_slice(&[42; 29]);
    DelegatedAuthMetrics::record_authority(principal);
    DelegatedAuthMetrics::record_root_proof_prepare_completed();
    DelegatedAuthMetrics::record_issuer_proof_prepare_completed();
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
        &["delegated_auth", "prepare_issuer_proof", "completed", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["delegated_auth", "verify_token", "started", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["delegated_auth", "verify_token", "completed", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["delegated_auth", "verify_token", "failed", "token_expired"],
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
        AuthMetricOperation::Establish,
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
    ManagementCallMetrics::record(
        ManagementCallMetricOperation::InstallCode,
        ManagementCallMetricOutcome::Started,
        ManagementCallMetricReason::Ok,
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
        WasmStoreMetricOperation::SourceResolve,
        WasmStoreMetricSource::Resolver,
        WasmStoreMetricOutcome::Completed,
        WasmStoreMetricReason::Ok,
    );
    perf::record_checkpoint("metrics::tests", "checkpoint", 7);
}

#[test]
fn metrics_docs_cover_all_metric_families() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let docs_path = workspace_root.join("docs/metrics.md");
    let git_marker = workspace_root.join(".git");

    if !docs_path.exists() && !git_marker.exists() {
        return;
    }

    let docs = std::fs::read_to_string(&docs_path).unwrap_or_else(|error| {
        let docs_display = docs_path.display();
        panic!("failed to read {docs_display}: {error}");
    });

    for kind in all_metric_kinds() {
        let name = kind.metric_family_name_for_tests();
        let table_row = format!("| `{name}` |");
        let detail_header = format!("### `{name}`");

        assert!(
            docs.contains(&table_row),
            "docs/metrics.md table should include MetricsKind::{name}"
        );
        assert!(
            docs.contains(&detail_header),
            "docs/metrics.md details should include MetricsKind::{name}"
        );
    }
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

trait MetricsKindTestName {
    fn metric_family_name_for_tests(self) -> &'static str;
}

impl MetricsKindTestName for MetricsKind {
    fn metric_family_name_for_tests(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Placement => "Placement",
            Self::Platform => "Platform",
            Self::Runtime => "Runtime",
            Self::Security => "Security",
            Self::Storage => "Storage",
        }
    }
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

fn assert_metric_count_and_u64(
    entries: &[MetricEntry],
    labels: &[&str],
    expected_count: u64,
    expected_value: u64,
) {
    let entry = entries
        .iter()
        .find(|entry| entry.labels.iter().map(String::as_str).collect::<Vec<_>>() == labels)
        .expect("metric entry should exist");

    match &entry.value {
        MetricValue::CountAndU64 { count, value_u64 } => {
            assert_eq!(*count, expected_count);
            assert_eq!(*value_u64, expected_value);
        }
        _ => panic!("metric entry should use CountAndU64"),
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
        MetricValue::U128(value) => assert_eq!(*value, expected),
        _ => panic!("metric entry should use U128"),
    }
}
