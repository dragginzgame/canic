pub mod icts;

use crate::{
    PublicError, api,
    cdk::{
        spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
        types::{Cycles, Principal},
    },
    dto::{
        canister::{CanisterStatusView, CanisterSummaryView},
        cascade::{StateSnapshotView, TopologySnapshotView},
        env::EnvView,
        log::LogEntryView,
        memory::MemoryRegistryView,
        metrics::{
            AccessMetricEntry, EndpointHealthView, HttpMetricEntry, IccMetricEntry,
            SystemMetricEntry, TimerMetricEntry,
        },
        page::{Page, PageRequest},
        placement::{ScalingRegistryView, ShardingRegistryView, ShardingTenantsView},
        pool::{CanisterPoolView, PoolAdminCommand, PoolAdminResponse},
        registry::{AppRegistryView, SubnetRegistryView},
        rpc::{Request, Response, UpgradeCanisterResponse},
        state::{AppCommand, AppStateView, SubnetStateView},
    },
    ids::CanisterRole,
    log::Level,
    ops,
    perf::PerfEntry,
    workflow,
};

//
// ICRC
//

pub fn icrc10_supported_standards() -> Result<Vec<(String, String)>, PublicError> {
    Ok(workflow::icrc::query::icrc10_supported_standards())
}

pub fn icrc21_canister_call_consent_message(
    req: ConsentMessageRequest,
) -> Result<ConsentMessageResponse, PublicError> {
    Ok(workflow::icrc::query::icrc21_consent_message(req))
}

//
// CANISTER HELPERS
//

pub fn canic_memory_registry() -> Result<MemoryRegistryView, PublicError> {
    Ok(workflow::memory::query::memory_registry_view())
}

pub fn canic_env() -> Result<EnvView, PublicError> {
    Ok(workflow::env::query::env_view())
}

pub fn canic_log(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Result<Page<LogEntryView>, PublicError> {
    Ok(workflow::log::query::log_page(
        crate_name, topic, min_level, page,
    ))
}

//
// METRICS
//

pub fn canic_metrics_system() -> Result<Vec<SystemMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_system_snapshot())
}

pub fn canic_metrics_icc(page: PageRequest) -> Result<Page<IccMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_icc_page(page))
}

pub fn canic_metrics_http(page: PageRequest) -> Result<Page<HttpMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_http_page(page))
}

pub fn canic_metrics_timer(page: PageRequest) -> Result<Page<TimerMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_timer_page(page))
}

pub fn canic_metrics_access(page: PageRequest) -> Result<Page<AccessMetricEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_access_page(page))
}

pub fn canic_metrics_perf(page: PageRequest) -> Result<Page<PerfEntry>, PublicError> {
    Ok(workflow::metrics::query::metrics_perf_page(page))
}

pub fn canic_metrics_endpoint_health(
    page: PageRequest,
) -> Result<Page<EndpointHealthView>, PublicError> {
    Ok(workflow::metrics::query::metrics_endpoint_health_page(
        page,
        Some("canic_metrics_endpoint_health"),
    ))
}

//
// STATE
//

pub fn canic_app_state() -> Result<AppStateView, PublicError> {
    Ok(workflow::state::query::app_state_view())
}

pub fn canic_subnet_state() -> Result<SubnetStateView, PublicError> {
    Ok(workflow::state::query::subnet_state_view())
}

//
// REGISTRIES
//

pub fn canic_app_registry() -> Result<AppRegistryView, PublicError> {
    Ok(workflow::registry::query::app_registry_view())
}

pub fn canic_subnet_registry() -> Result<SubnetRegistryView, PublicError> {
    Ok(workflow::registry::query::subnet_registry_view())
}

//
// DIRECTORIES
//

pub fn canic_app_directory(
    page: PageRequest,
) -> Result<Page<(CanisterRole, Principal)>, PublicError> {
    Ok(workflow::directory::query::app_directory_page(page))
}

pub fn canic_subnet_directory(
    page: PageRequest,
) -> Result<Page<(CanisterRole, Principal)>, PublicError> {
    Ok(workflow::directory::query::subnet_directory_page(page))
}

//
// TOPOLOGY
//

pub fn canic_canister_children(
    page: PageRequest,
) -> Result<Page<CanisterSummaryView>, PublicError> {
    Ok(workflow::children::query::canister_children_page(page))
}

//
// CYCLES
//

pub fn canic_cycle_tracker(page: PageRequest) -> Result<Page<(u64, Cycles)>, PublicError> {
    Ok(workflow::runtime::cycles::query::cycle_tracker_page(page))
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

pub fn canic_sharding_tenants(
    pool: String,
    shard_pid: Principal,
) -> Result<ShardingTenantsView, PublicError> {
    Ok(workflow::placement::query::sharding_tenants_view(
        &pool, shard_pid,
    ))
}

//
// ROOT-ONLY ENDPOINTS
//

pub async fn canic_app(cmd: AppCommand) -> Result<(), PublicError> {
    api::app::apply_command(cmd).await
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

pub async fn canic_canister_status(pid: Principal) -> Result<CanisterStatusView, PublicError> {
    ops::ic::mgmt::canister_status(pid)
        .await
        .map_err(PublicError::from)
}

pub fn canic_config() -> Result<String, PublicError> {
    api::config::export_toml()
}

pub fn canic_pool_list() -> Result<CanisterPoolView, PublicError> {
    Ok(workflow::pool::query::pool_list_view())
}

pub async fn canic_pool_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, PublicError> {
    workflow::pool::admin::handle_admin(cmd)
        .await
        .map_err(PublicError::from)
}

//
// SYNC
//

pub async fn canic_sync_state(view: StateSnapshotView) -> Result<(), PublicError> {
    workflow::cascade::state::nonroot_cascade_state(view)
        .await
        .map_err(PublicError::from)
}

pub async fn canic_sync_topology(view: TopologySnapshotView) -> Result<(), PublicError> {
    workflow::cascade::topology::nonroot_cascade_topology(view)
        .await
        .map_err(PublicError::from)
}
