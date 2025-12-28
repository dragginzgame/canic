use crate::{dto::metrics::system::SystemMetricEntry, model::metrics::system::SystemMetricKind};

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
    use SystemMetricKind::*;
    match kind {
        CanisterCall => "CanisterCall",
        CanisterStatus => "CanisterStatus",
        CreateCanister => "CreateCanister",
        DeleteCanister => "DeleteCanister",
        DepositCycles => "DepositCycles",
        HttpOutcall => "HttpOutcall",
        InstallCode => "InstallCode",
        ReinstallCode => "ReinstallCode",
        TimerScheduled => "TimerScheduled",
        UninstallCode => "UninstallCode",
        UpdateSettings => "UpdateSettings",
        UpgradeCode => "UpgradeCode",
    }
    .to_string()
}
