use crate::{
    PublicError,
    cdk::{
        api::canister_self,
        mgmt::CanisterStatusResult,
        spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
        types::{Cycles, Principal},
    },
    dto::{
        canister::CanisterSummaryView,
        env::EnvView,
        log::LogEntryView,
        memory::MemoryRegistryView,
        metrics::{
            AccessMetricEntry, EndpointHealthView, HttpMetricEntry, IccMetricEntry,
            TimerMetricEntry,
        },
        page::{Page, PageRequest},
        placement::{ScalingRegistryView, ShardingRegistryView},
        pool::{CanisterPoolView, PoolAdminCommand, PoolAdminResponse},
        registry::{AppRegistryView, SubnetRegistryView},
        rpc::{Request, Response, UpgradeCanisterResponse},
        snapshot::{StateSnapshotView, TopologySnapshotView},
        state::{AppCommand, AppStateView, SubnetStateView},
    },
    ids::CanisterRole,
    log::Level,
    ops::{
        icrc::{Icrc10Ops, Icrc21Ops},
        runtime::{
            env::EnvOps,
            log::LogViewOps,
            memory::MemoryOps,
            metrics::{MetricsOps, SystemMetricsSnapshot},
        },
        storage::{
            children::CanisterChildrenOps,
            cycles::CycleTrackerOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            pool::PoolOps,
            registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
            scaling::ScalingRegistryOps,
            sharding::ShardingRegistryOps,
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    perf::PerfEntry,
    workflow,
};

//
// ICRC
//

#[must_use]
pub fn icrc10_supported_standards() -> Vec<(String, String)> {
    Icrc10Ops::supported_standards()
}

#[must_use]
pub fn icrc21_canister_call_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
    Icrc21Ops::consent_message(req)
}

//
// CANISTER HELPERS
//

#[must_use]
pub fn canic_memory_registry() -> MemoryRegistryView {
    MemoryOps::export_view()
}

#[must_use]
pub fn canic_env() -> EnvView {
    EnvOps::export_view()
}

#[must_use]
pub fn canic_log(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Page<LogEntryView> {
    LogViewOps::page(crate_name, topic, min_level, page)
}

//
// METRICS
//

#[must_use]
pub fn canic_metrics_system() -> SystemMetricsSnapshot {
    MetricsOps::system_snapshot()
}

#[must_use]
pub fn canic_metrics_icc(page: PageRequest) -> Page<IccMetricEntry> {
    MetricsOps::icc_page(page)
}

#[must_use]
pub fn canic_metrics_http(page: PageRequest) -> Page<HttpMetricEntry> {
    MetricsOps::http_page(page)
}

#[must_use]
pub fn canic_metrics_timer(page: PageRequest) -> Page<TimerMetricEntry> {
    MetricsOps::timer_page(page)
}

#[must_use]
pub fn canic_metrics_access(page: PageRequest) -> Page<AccessMetricEntry> {
    MetricsOps::access_page(page)
}

#[must_use]
pub fn canic_metrics_perf(page: PageRequest) -> Page<PerfEntry> {
    crate::ops::perf::PerfOps::snapshot(page)
}

#[must_use]
pub fn canic_metrics_endpoint_health(page: PageRequest) -> Page<EndpointHealthView> {
    MetricsOps::endpoint_health_page_excluding(page, Some("canic_metrics_endpoint_health"))
}

//
// STATE
//

#[must_use]
pub fn canic_app_state() -> AppStateView {
    AppStateOps::export_view()
}

#[must_use]
pub fn canic_subnet_state() -> SubnetStateView {
    SubnetStateOps::export_view()
}

//
// REGISTRIES
//

#[must_use]
pub fn canic_app_registry() -> AppRegistryView {
    AppRegistryOps::export_view()
}

#[must_use]
pub fn canic_subnet_registry() -> SubnetRegistryView {
    SubnetRegistryOps::export_view()
}

//
// DIRECTORY VIEWS
//

#[must_use]
pub fn canic_app_directory(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    AppDirectoryOps::page(page)
}

#[must_use]
pub fn canic_subnet_directory(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    SubnetDirectoryOps::page(page)
}

//
// TOPOLOGY
//

#[must_use]
pub fn canic_subnet_canister_children(page: PageRequest) -> Page<CanisterSummaryView> {
    CanisterChildrenOps::page(page)
}

//
// CYCLES
//

#[must_use]
pub fn canic_cycle_tracker(page: PageRequest) -> Page<(u64, Cycles)> {
    CycleTrackerOps::page(page)
}

//
// SCALING
//

pub fn canic_scaling_registry() -> Result<ScalingRegistryView, PublicError> {
    Ok(ScalingRegistryOps::export_view())
}

//
// SHARDING
//

pub fn canic_sharding_registry() -> Result<ShardingRegistryView, PublicError> {
    Ok(ShardingRegistryOps::export_view())
}

//
// ROOT ENDPOINTS
//

pub async fn canic_app(cmd: AppCommand) -> Result<(), PublicError> {
    workflow::app::AppStateOrchestrator::apply_command(cmd).await
}

pub async fn canic_canister_upgrade(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, PublicError> {
    workflow::rpc::client::upgrade_canister_request(canister_pid)
        .await
        .map_err(PublicError::from)
}

pub async fn canic_response(request: Request) -> Result<Response, PublicError> {
    workflow::rpc::handler::response(request).await
}

pub async fn canic_canister_status(pid: Principal) -> Result<CanisterStatusResult, PublicError> {
    crate::ops::ic::mgmt::canister_status(pid)
        .await
        .map_err(PublicError::from)
}

pub fn canic_config() -> Result<String, PublicError> {
    workflow::config::export_toml()
}

pub fn canic_pool_list() -> Result<CanisterPoolView, PublicError> {
    Ok(PoolOps::export_view())
}

pub async fn canic_pool_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, PublicError> {
    workflow::pool::admin::handle_admin(cmd).await
}

//
// SYNC
//

pub async fn canic_sync_state(snapshot: StateSnapshotView) -> Result<(), PublicError> {
    workflow::cascade::state::nonroot_cascade_state(&snapshot).await
}

pub async fn canic_sync_topology(snapshot: TopologySnapshotView) -> Result<(), PublicError> {
    workflow::cascade::topology::nonroot_cascade_topology(&snapshot).await
}

//
// ICTS
//

pub async fn icts_canister_status() -> Result<CanisterStatusResult, String> {
    crate::ops::ic::mgmt::canister_status(canister_self())
        .await
        .map_err(|err| err.to_string())
}
