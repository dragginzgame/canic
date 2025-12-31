use crate::{dto::metrics::SystemMetricEntry, storage::metrics::system::SystemMetricKind};

#[must_use]
pub fn system_metrics_to_view(
    raw: impl IntoIterator<Item = (SystemMetricKind, u64)>,
) -> Vec<SystemMetricEntry> {
    raw.into_iter()
        .map(|(kind, count)| SystemMetricEntry {
            kind: kind_to_string(kind),
            count,
        })
        .collect()
}

fn kind_to_string(kind: SystemMetricKind) -> String {
    match kind {
        SystemMetricKind::CanisterCall => "CanisterCall",
        SystemMetricKind::CanisterStatus => "CanisterStatus",
        SystemMetricKind::CreateCanister => "CreateCanister",
        SystemMetricKind::DeleteCanister => "DeleteCanister",
        SystemMetricKind::DepositCycles => "DepositCycles",
        SystemMetricKind::HttpOutcall => "HttpOutcall",
        SystemMetricKind::InstallCode => "InstallCode",
        SystemMetricKind::RawRand => "RawRand",
        SystemMetricKind::ReinstallCode => "ReinstallCode",
        SystemMetricKind::TimerScheduled => "TimerScheduled",
        SystemMetricKind::UninstallCode => "UninstallCode",
        SystemMetricKind::UpdateSettings => "UpdateSettings",
        SystemMetricKind::UpgradeCode => "UpgradeCode",
    }
    .to_string()
}
