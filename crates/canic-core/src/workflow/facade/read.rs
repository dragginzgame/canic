use crate::{
    Error,
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
        pool::CanisterPoolView,
        registry::{AppRegistryView, SubnetRegistryView},
        state::{AppStateView, SubnetStateView},
    },
    ids::CanisterRole,
    log::Level,
    ops::{
        ic::mgmt,
        icrc::{Icrc10Ops, Icrc21Ops},
        perf::PerfOps,
        runtime::{env::EnvOps, log::LogViewOps, memory::MemoryOps, metrics::MetricsOps},
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
};

pub(crate) fn icrc10_supported_standards() -> Vec<(String, String)> {
    Icrc10Ops::supported_standards()
}

pub(crate) fn icrc21_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
    Icrc21Ops::consent_message(req)
}

pub(crate) fn memory_registry() -> MemoryRegistryView {
    MemoryOps::export_view()
}

pub(crate) fn env_view() -> EnvView {
    EnvOps::export_view()
}

pub(crate) fn log_page(
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
) -> Page<LogEntryView> {
    LogViewOps::page(crate_name, topic, min_level, page)
}

pub(crate) fn metrics_system_snapshot() -> Vec<SystemMetricEntry> {
    MetricsOps::system_snapshot()
}

pub(crate) fn metrics_icc_page(page: PageRequest) -> Page<IccMetricEntry> {
    MetricsOps::icc_page(page)
}

pub(crate) fn metrics_http_page(page: PageRequest) -> Page<HttpMetricEntry> {
    MetricsOps::http_page(page)
}

pub(crate) fn metrics_timer_page(page: PageRequest) -> Page<TimerMetricEntry> {
    MetricsOps::timer_page(page)
}

pub(crate) fn metrics_access_page(page: PageRequest) -> Page<AccessMetricEntry> {
    MetricsOps::access_page(page)
}

pub(crate) fn metrics_perf_page(page: PageRequest) -> Page<PerfEntry> {
    PerfOps::snapshot(page)
}

pub(crate) fn metrics_endpoint_health_page(
    page: PageRequest,
    exclude_endpoint: Option<&str>,
) -> Page<EndpointHealthView> {
    MetricsOps::endpoint_health_page_excluding(page, exclude_endpoint)
}

pub(crate) fn app_state_view() -> AppStateView {
    AppStateOps::export_view()
}

pub(crate) fn subnet_state_view() -> SubnetStateView {
    SubnetStateOps::export_view()
}

pub(crate) fn app_registry_view() -> AppRegistryView {
    AppRegistryOps::export_view()
}

pub(crate) fn subnet_registry_view() -> SubnetRegistryView {
    SubnetRegistryOps::export_view()
}

pub(crate) fn app_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    AppDirectoryOps::page(page)
}

pub(crate) fn subnet_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    SubnetDirectoryOps::page(page)
}

pub(crate) fn subnet_canister_children_page(page: PageRequest) -> Page<CanisterSummaryView> {
    CanisterChildrenOps::page(page)
}

pub(crate) fn cycle_tracker_page(page: PageRequest) -> Page<(u64, Cycles)> {
    CycleTrackerOps::page(page)
}

pub(crate) fn scaling_registry_view() -> ScalingRegistryView {
    ScalingRegistryOps::export_view()
}

pub(crate) fn sharding_registry_view() -> ShardingRegistryView {
    ShardingRegistryOps::export_view()
}

pub(crate) fn pool_list_view() -> CanisterPoolView {
    PoolOps::export_view()
}

pub(crate) async fn canister_status(pid: Principal) -> Result<CanisterStatusResult, Error> {
    mgmt::canister_status(pid).await
}
