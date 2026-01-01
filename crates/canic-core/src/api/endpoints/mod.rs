pub mod icts;

use crate::{
    PublicError, api,
    cdk::{
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
            SystemMetricEntry, TimerMetricEntry,
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
    perf::PerfEntry,
    workflow,
};

//
// ICRC
//

#[must_use]
pub fn icrc10_supported_standards() -> Vec<(String, String)> {
    workflow::icrc::query::icrc10_supported_standards()
}

#[must_use]
pub fn icrc21_canister_call_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
    workflow::icrc::query::icrc21_consent_message(req)
}

//
// CANISTER HELPERS
//

#[must_use]
pub fn canic_memory_registry() -> MemoryRegistryView {
    workflow::memory::query::memory_registry_view()
}

#[must_use]
pub fn canic_env() -> EnvView {
    workflow::env::query::env_view()
}

#[must_use]
pub fn canic_log(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Page<LogEntryView> {
    workflow::log::query::log_page(crate_name, topic, min_level, page)
}

//
// METRICS
//

#[must_use]
pub fn canic_metrics_system() -> Vec<SystemMetricEntry> {
    workflow::metrics::query::metrics_system_snapshot()
}

#[must_use]
pub fn canic_metrics_icc(page: PageRequest) -> Page<IccMetricEntry> {
    workflow::metrics::query::metrics_icc_page(page)
}

#[must_use]
pub fn canic_metrics_http(page: PageRequest) -> Page<HttpMetricEntry> {
    workflow::metrics::query::metrics_http_page(page)
}

#[must_use]
pub fn canic_metrics_timer(page: PageRequest) -> Page<TimerMetricEntry> {
    workflow::metrics::query::metrics_timer_page(page)
}

#[must_use]
pub fn canic_metrics_access(page: PageRequest) -> Page<AccessMetricEntry> {
    workflow::metrics::query::metrics_access_page(page)
}

#[must_use]
pub fn canic_metrics_perf(page: PageRequest) -> Page<PerfEntry> {
    workflow::metrics::query::metrics_perf_page(page)
}

#[must_use]
pub fn canic_metrics_endpoint_health(page: PageRequest) -> Page<EndpointHealthView> {
    workflow::metrics::query::metrics_endpoint_health_page(
        page,
        Some("canic_metrics_endpoint_health"),
    )
}

//
// STATE
//

#[must_use]
pub fn canic_app_state() -> AppStateView {
    workflow::state::query::app_state_view()
}

#[must_use]
pub fn canic_subnet_state() -> SubnetStateView {
    workflow::state::query::subnet_state_view()
}

//
// REGISTRIES
//

#[must_use]
pub fn canic_app_registry() -> AppRegistryView {
    workflow::registry::query::app_registry_view()
}

#[must_use]
pub fn canic_subnet_registry() -> SubnetRegistryView {
    workflow::registry::query::subnet_registry_view()
}

//
// DIRECTORY VIEWS
//

#[must_use]
pub fn canic_app_directory(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    workflow::directory::query::app_directory_page(page)
}

#[must_use]
pub fn canic_subnet_directory(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    workflow::directory::query::subnet_directory_page(page)
}

//
// TOPOLOGY
//

#[must_use]
pub fn canic_canister_children(page: PageRequest) -> Page<CanisterSummaryView> {
    workflow::children::query::canister_children_page(page)
}

//
// CYCLES
//

#[must_use]
pub fn canic_cycle_tracker(page: PageRequest) -> Page<(u64, Cycles)> {
    workflow::runtime::cycles::query::cycle_tracker_page(page)
}

//
// SCALING
//

pub fn canic_scaling_registry() -> Result<ScalingRegistryView, PublicError> {
    Ok(workflow::placement::query::scaling_registry_view())
}

//
// SHARDING
//

pub fn canic_sharding_registry() -> Result<ShardingRegistryView, PublicError> {
    Ok(workflow::placement::query::sharding_registry_view())
}

//
// ROOT ENDPOINTS
//

pub async fn canic_app(cmd: AppCommand) -> Result<(), PublicError> {
    crate::api::app::apply_command(cmd).await
}

pub async fn canic_canister_upgrade(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, PublicError> {
    workflow::rpc::request::upgrade_canister_request(canister_pid)
        .await
        .map_err(PublicError::from)
}

pub async fn canic_response(request: Request) -> Result<Response, PublicError> {
    workflow::rpc::request::handler::response(request)
        .await
        .map_err(PublicError::from)
}

pub async fn canic_canister_status(pid: Principal) -> Result<CanisterStatusResult, PublicError> {
    workflow::canister::canister_status(pid)
        .await
        .map_err(PublicError::from)
}

pub fn canic_config() -> Result<String, PublicError> {
    api::config::export_toml()
}

pub fn canic_pool_list() -> Result<CanisterPoolView, PublicError> {
    Ok(workflow::view::pool::pool_list_view())
}

pub async fn canic_pool_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, PublicError> {
    workflow::pool::admin::handle_admin(cmd)
        .await
        .map_err(PublicError::from)
}

//
// SYNC
//

pub async fn canic_sync_state(snapshot: StateSnapshotView) -> Result<(), PublicError> {
    workflow::cascade::state::nonroot_cascade_state(&snapshot)
        .await
        .map_err(PublicError::from)
}

pub async fn canic_sync_topology(snapshot: TopologySnapshotView) -> Result<(), PublicError> {
    workflow::cascade::topology::nonroot_cascade_topology(&snapshot)
        .await
        .map_err(PublicError::from)
}
