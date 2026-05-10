use super::*;
#[cfg(feature = "sharding")]
use crate::ops::runtime::metrics::sharding::{
    ShardingMetricOperation, ShardingMetricOutcome, ShardingMetricReason, ShardingMetrics,
};
use crate::{
    cdk::types::Principal,
    ids::{AccessMetricKind, CanisterRole},
    ops::runtime::metrics::{
        auth::{AuthMetricOperation, AuthMetricOutcome, AuthMetricReason, AuthMetricSurface},
        canister_ops::{
            CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
        },
        cascade::{CascadeMetricOperation, CascadeMetricOutcome, CascadeMetricReason},
        cycles_funding::CyclesFundingDeniedReason,
        delegated_auth::{
            DelegatedAuthMetricOperation, DelegatedAuthMetricOutcome, DelegatedAuthMetricReason,
        },
        directory::{DirectoryMetricOperation, DirectoryMetricOutcome, DirectoryMetricReason},
        http::HttpMethod,
        intent::{
            IntentMetricOperation, IntentMetricOutcome, IntentMetricReason, IntentMetricSurface,
        },
        lifecycle::{
            LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricStage,
        },
        management_call::{
            ManagementCallMetricOperation, ManagementCallMetricOutcome, ManagementCallMetricReason,
        },
        platform_call::{
            PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
            PlatformCallMetricSurface,
        },
        pool::{PoolMetricOperation, PoolMetricOutcome, PoolMetricReason},
        provisioning::{
            ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
        },
        replay::{ReplayMetricOperation, ReplayMetricOutcome, ReplayMetricReason},
        root_capability::{
            RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetricProofMode,
        },
        scaling::{ScalingMetricOperation, ScalingMetricOutcome, ScalingMetricReason},
        timer::TimerMode,
        wasm_store::{
            WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason,
            WasmStoreMetricSource,
        },
    },
};
use std::time::Duration;

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

    let entries = entries(MetricsKind::Security);

    assert_metric_count(
        &entries,
        &["auth", "session", "bootstrap", "rejected", "token_invalid"],
        1,
    );
    assert_metric_count(
        &entries,
        &["auth", "session", "session", "completed", "created"],
        2,
    );
    assert_metric_count(
        &entries,
        &["auth", "attestation", "verify", "failed", "unknown_key_id"],
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
            "upgrade",
            "worker",
            "failed",
            "management_call",
        ],
        2,
    );
    assert_metric_count(
        &entries,
        &[
            "canister_ops",
            "create",
            "worker",
            "completed",
            "pool_reuse",
        ],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "canister_ops",
            "create",
            "worker",
            "failed",
            "state_propagation",
        ],
        1,
    );
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

    let entries = entries(MetricsKind::Placement);

    assert_metric_count(&entries, &["directory", "resolve", "started", "ok"], 1);
    assert_metric_count(
        &entries,
        &["directory", "classify", "completed", "pending_fresh"],
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

    let entries = entries(MetricsKind::Placement);

    assert_metric_count(&entries, &["pool", "reset", "started", "ok"], 1);
    assert_metric_count(
        &entries,
        &["pool", "import_queued", "skipped", "already_present"],
        2,
    );
}

#[test]
fn provisioning_metrics_remain_internal_workflow_counters() {
    reset_for_tests();

    ProvisioningMetrics::record(
        ProvisioningMetricOperation::ResolveModule,
        &CanisterRole::new("app"),
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    ProvisioningMetrics::record(
        ProvisioningMetricOperation::Install,
        &CanisterRole::new("worker"),
        ProvisioningMetricOutcome::Failed,
        ProvisioningMetricReason::MissingWasm,
    );
    ProvisioningMetrics::record(
        ProvisioningMetricOperation::Install,
        &CanisterRole::new("worker"),
        ProvisioningMetricOutcome::Failed,
        ProvisioningMetricReason::MissingWasm,
    );

    let snapshot = ProvisioningMetrics::snapshot();
    assert_eq!(snapshot.len(), 2);
    assert!(snapshot.iter().any(|(key, count)| key.operation
        == ProvisioningMetricOperation::ResolveModule
        && key.role == "app"
        && key.outcome == ProvisioningMetricOutcome::Started
        && key.reason == ProvisioningMetricReason::Ok
        && *count == 1));
    assert!(snapshot.iter().any(|(key, count)| key.operation
        == ProvisioningMetricOperation::Install
        && key.role == "worker"
        && key.outcome == ProvisioningMetricOutcome::Failed
        && key.reason == ProvisioningMetricReason::MissingWasm
        && *count == 2));
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
fn platform_call_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Generic,
        PlatformCallMetricMode::BoundedWait,
        PlatformCallMetricOutcome::Started,
        PlatformCallMetricReason::Ok,
    );
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Ledger,
        PlatformCallMetricMode::Update,
        PlatformCallMetricOutcome::Failed,
        PlatformCallMetricReason::LedgerRejected,
    );
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Ledger,
        PlatformCallMetricMode::Update,
        PlatformCallMetricOutcome::Failed,
        PlatformCallMetricReason::LedgerRejected,
    );

    let entries = entries(MetricsKind::Platform);

    assert_metric_count(
        &entries,
        &["platform_call", "generic", "bounded_wait", "started", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &[
            "platform_call",
            "ledger",
            "update",
            "failed",
            "ledger_rejected",
        ],
        2,
    );
}

#[test]
fn intent_metrics_are_exposed_with_stable_labels() {
    reset_for_tests();

    IntentMetrics::record(
        IntentMetricSurface::Call,
        IntentMetricOperation::Reserve,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    IntentMetrics::record(
        IntentMetricSurface::Call,
        IntentMetricOperation::Commit,
        IntentMetricOutcome::Failed,
        IntentMetricReason::StorageFailed,
    );
    IntentMetrics::record(
        IntentMetricSurface::Call,
        IntentMetricOperation::Commit,
        IntentMetricOutcome::Failed,
        IntentMetricReason::StorageFailed,
    );

    let entries = entries(MetricsKind::Runtime);

    assert_metric_count(
        &entries,
        &["intent", "call", "reserve", "completed", "ok"],
        1,
    );
    assert_metric_count(
        &entries,
        &["intent", "call", "commit", "failed", "storage_failed"],
        2,
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

    let entries = entries(MetricsKind::Security);

    assert_metric_count(&entries, &["delegated_auth", "delegated_auth_authority"], 1);
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
        assert!(entries(*kind).is_empty());
    }
}

fn seed_all_metric_families_for_reset_test() {
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
        crate::ops::runtime::metrics::cascade::CascadeMetricSnapshot::State,
        CascadeMetricOutcome::Started,
        CascadeMetricReason::Ok,
    );
    CyclesFundingMetrics::record_denied(principal, 10, CyclesFundingDeniedReason::ChildNotFound);
    CyclesTopupMetrics::record_request_scheduled();
    DelegatedAuthMetrics::record_authority(principal);
    DirectoryMetrics::record(
        DirectoryMetricOperation::Resolve,
        DirectoryMetricOutcome::Started,
        DirectoryMetricReason::Ok,
    );
    HttpMetrics::record_http_request(HttpMethod::Get, "https://example.test/a", Some("api"));
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Generic,
        PlatformCallMetricMode::BoundedWait,
        PlatformCallMetricOutcome::Started,
        PlatformCallMetricReason::Ok,
    );
    InterCanisterCallMetrics::record_call(principal, "canic_sync");
    IntentMetrics::record(
        IntentMetricSurface::Call,
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
    PoolMetrics::record(
        PoolMetricOperation::Reset,
        PoolMetricOutcome::Started,
        PoolMetricReason::Ok,
    );
    ProvisioningMetrics::record(
        ProvisioningMetricOperation::Create,
        &CanisterRole::new("app"),
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
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
