use crate::{
    InternalError,
    ops::{
        ic::mgmt::MgmtOps,
        runtime::{
            env::EnvOps,
            metrics::canister_ops::{
                CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
                CanisterOpsMetrics,
            },
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::{
        ic::provision::{ProvisionWorkflow, metrics::record_delete_metric},
        prelude::*,
    },
};

impl ProvisionWorkflow {
    /// Delete an existing canister.
    ///
    /// PHASES:
    /// 0. Uninstall code.
    /// 1. Stop and delete via management canister.
    /// 2. Remove from SubnetRegistry.
    pub async fn uninstall_and_delete_canister(pid: Principal) -> Result<(), InternalError> {
        if let Err(err) = EnvOps::require_root() {
            CanisterOpsMetrics::record_unscoped(
                CanisterOpsMetricOperation::Delete,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        let role = SubnetRegistryOps::get(pid).map(|record| record.role);
        record_delete_metric(
            role.as_ref(),
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );

        if let Err(err) = MgmtOps::uninstall_code(pid).await {
            record_delete_metric(
                role.as_ref(),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        if let Err(err) = MgmtOps::stop_canister(pid).await {
            record_delete_metric(
                role.as_ref(),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        if let Err(err) = MgmtOps::delete_canister(pid).await {
            record_delete_metric(
                role.as_ref(),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        let removed_entry = SubnetRegistryOps::remove(&pid);
        match &removed_entry {
            Some(c) => log!(
                Topic::CanisterLifecycle,
                Ok,
                "🗑️ delete_canister: {} ({})",
                pid,
                c.role
            ),
            None => log!(
                Topic::CanisterLifecycle,
                Warn,
                "🗑️ delete_canister: {pid} not in registry"
            ),
        }

        record_delete_metric(
            role.as_ref(),
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );

        Ok(())
    }
}
