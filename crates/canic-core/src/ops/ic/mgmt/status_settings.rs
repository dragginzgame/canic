use super::*;

impl MgmtOps {
    /// Internal ops entrypoint used by workflow and other ops helpers.
    pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatus, InternalError> {
        let status = management_call(
            ManagementCallMetricOperation::CanisterStatus,
            MgmtInfra::canister_status(canister_pid),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::CanisterStatus);

        Ok(canister_status_from_infra(status))
    }

    /// Updates canister settings via the management canister and records metrics.
    pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), InternalError> {
        let infra_args = update_settings_to_infra(args);
        management_call(
            ManagementCallMetricOperation::UpdateSettings,
            MgmtInfra::update_settings(&infra_args),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::UpdateSettings);

        Ok(())
    }
}
