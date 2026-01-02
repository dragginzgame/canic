use crate::ops::runtime::metrics::store::system::{
    SystemMetricKind as ModelSystemMetricKind, SystemMetrics,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum SystemMetricKind {
    CanisterCall,
    CanisterStatus,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    HttpOutcall,
    InstallCode,
    RawRand,
    ReinstallCode,
    TimerScheduled,
    UninstallCode,
    UpdateSettings,
    UpgradeCode,
}

#[derive(Clone, Debug)]
pub struct SystemMetricsSnapshot {
    pub entries: Vec<(SystemMetricKind, u64)>,
}

#[must_use]
pub fn snapshot() -> SystemMetricsSnapshot {
    let entries = SystemMetrics::export_raw()
        .into_iter()
        .map(|(kind, count)| (kind_from_model(kind), count))
        .collect();
    SystemMetricsSnapshot { entries }
}

/// Record a single system metric.
pub fn record_system_metric(kind: SystemMetricKind) {
    SystemMetrics::increment(kind_to_model(kind));
}

/// Record a single HTTP outcall for system metrics.
pub fn record_http_outcall() {
    record_system_metric(SystemMetricKind::HttpOutcall);
}

const fn kind_from_model(kind: ModelSystemMetricKind) -> SystemMetricKind {
    match kind {
        ModelSystemMetricKind::CanisterCall => SystemMetricKind::CanisterCall,
        ModelSystemMetricKind::CanisterStatus => SystemMetricKind::CanisterStatus,
        ModelSystemMetricKind::CreateCanister => SystemMetricKind::CreateCanister,
        ModelSystemMetricKind::DeleteCanister => SystemMetricKind::DeleteCanister,
        ModelSystemMetricKind::DepositCycles => SystemMetricKind::DepositCycles,
        ModelSystemMetricKind::HttpOutcall => SystemMetricKind::HttpOutcall,
        ModelSystemMetricKind::InstallCode => SystemMetricKind::InstallCode,
        ModelSystemMetricKind::RawRand => SystemMetricKind::RawRand,
        ModelSystemMetricKind::ReinstallCode => SystemMetricKind::ReinstallCode,
        ModelSystemMetricKind::TimerScheduled => SystemMetricKind::TimerScheduled,
        ModelSystemMetricKind::UninstallCode => SystemMetricKind::UninstallCode,
        ModelSystemMetricKind::UpdateSettings => SystemMetricKind::UpdateSettings,
        ModelSystemMetricKind::UpgradeCode => SystemMetricKind::UpgradeCode,
    }
}

const fn kind_to_model(kind: SystemMetricKind) -> ModelSystemMetricKind {
    match kind {
        SystemMetricKind::CanisterCall => ModelSystemMetricKind::CanisterCall,
        SystemMetricKind::CanisterStatus => ModelSystemMetricKind::CanisterStatus,
        SystemMetricKind::CreateCanister => ModelSystemMetricKind::CreateCanister,
        SystemMetricKind::DeleteCanister => ModelSystemMetricKind::DeleteCanister,
        SystemMetricKind::DepositCycles => ModelSystemMetricKind::DepositCycles,
        SystemMetricKind::HttpOutcall => ModelSystemMetricKind::HttpOutcall,
        SystemMetricKind::InstallCode => ModelSystemMetricKind::InstallCode,
        SystemMetricKind::RawRand => ModelSystemMetricKind::RawRand,
        SystemMetricKind::ReinstallCode => ModelSystemMetricKind::ReinstallCode,
        SystemMetricKind::TimerScheduled => ModelSystemMetricKind::TimerScheduled,
        SystemMetricKind::UninstallCode => ModelSystemMetricKind::UninstallCode,
        SystemMetricKind::UpdateSettings => ModelSystemMetricKind::UpdateSettings,
        SystemMetricKind::UpgradeCode => ModelSystemMetricKind::UpgradeCode,
    }
}
