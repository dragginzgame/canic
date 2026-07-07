//! Module: workflow::ic::provision::delete
//!
//! Responsibility: uninstall and delete provisioned canisters.
//! Does not own: endpoint authorization, subnet registry schema, or management call internals.
//! Boundary: validates root context, delegates management calls, and updates registry state.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::metrics::{
        CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
    },
    log,
    log::Topic,
    ops::{
        ic::mgmt::MgmtOps,
        runtime::{env::EnvOps, metrics::canister_ops::CanisterOpsMetrics},
        storage::registry::subnet::SubnetRegistryOps,
    },
    workflow::ic::provision::{ProvisionWorkflow, metrics::record_delete_metric},
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

        let role = SubnetRegistryOps::role_parent(pid).map(|(role, _)| role);
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

        let removed_role = SubnetRegistryOps::remove_and_return_role(&pid);
        match &removed_role {
            Some(removed_role) => log!(
                Topic::CanisterLifecycle,
                Ok,
                "🗑️ delete_canister: {} ({})",
                pid,
                removed_role
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
