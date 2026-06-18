//! Module: workflow::ic::provision
//!
//! Responsibility: orchestrate root canister provisioning and teardown.
//! Does not own: authorization, stable records, or pure placement policy.
//! Boundary: workflow calls ops and policy after endpoints authenticate input.

mod allocation;
mod delete;
mod indexes;
mod install;
mod metrics;
mod payload;
mod policy;

use crate::{
    InternalError, InternalErrorOrigin,
    ops::{
        cost_guard::CostGuardPermit,
        runtime::{
            install_source::ModuleSourceRuntimeApi,
            metrics::{
                canister_ops::{
                    CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
                },
                provisioning::{
                    ProvisioningMetricOperation, ProvisioningMetricOutcome,
                    ProvisioningMetricReason,
                },
            },
        },
    },
    workflow::{
        ic::provision::{
            allocation::{AllocationSource, allocate_canister},
            install::install_canister,
            metrics::{record_canister_op, record_provisioning},
        },
        pool::PoolWorkflow,
        prelude::*,
    },
};

///
/// ProvisionWorkflow
///

pub struct ProvisionWorkflow;

impl ProvisionWorkflow {
    /// Create and install a new canister of the requested type beneath `parent`.
    ///
    /// PHASES:
    /// 1. Allocate a canister ID and cycles.
    /// 2. Install WASM + bootstrap initial state.
    /// 3. Register canister in SubnetRegistry.
    /// 4. Cascade topology + sync directories.
    pub async fn create_and_install_canister(
        deployment_permit: &CostGuardPermit,
        role: &CanisterRole,
        parent_pid: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        record_provisioning(
            role,
            ProvisioningMetricOperation::ResolveModule,
            ProvisioningMetricOutcome::Started,
            ProvisioningMetricReason::Ok,
        );
        let module_source = match ModuleSourceRuntimeApi::approved_module_source(role).await {
            Ok(module_source) => {
                record_provisioning(
                    role,
                    ProvisioningMetricOperation::ResolveModule,
                    ProvisioningMetricOutcome::Completed,
                    ProvisioningMetricReason::Ok,
                );
                module_source
            }
            Err(err) => {
                record_canister_op(
                    role,
                    CanisterOpsMetricOperation::Install,
                    CanisterOpsMetricOutcome::Failed,
                    CanisterOpsMetricReason::MissingWasm,
                );
                record_provisioning(
                    role,
                    ProvisioningMetricOperation::ResolveModule,
                    ProvisioningMetricOutcome::Failed,
                    ProvisioningMetricReason::MissingWasm,
                );
                return Err(err);
            }
        };

        let (pid, source) = allocate_canister(deployment_permit, role, parent_pid).await?;

        if let Err(err) = install_canister(
            deployment_permit,
            pid,
            role,
            parent_pid,
            &module_source,
            extra_arg,
        )
        .await
        {
            log!(
                Topic::CanisterLifecycle,
                Error,
                "install failed for {pid} ({role}): {err}"
            );
            if source == AllocationSource::Pool {
                if let Err(recycle_err) = PoolWorkflow::pool_import_canister(pid).await {
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "failed to recycle pool canister after install failure: {pid} ({recycle_err})"
                    );
                }
            } else if let Err(delete_err) = Self::uninstall_and_delete_canister(pid).await {
                log!(
                    Topic::CanisterLifecycle,
                    Warn,
                    "failed to delete canister after install failure: {pid} ({delete_err})"
                );
            }

            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("failed to install canister {pid}: {err}"),
            ));
        }

        Ok(pid)
    }
}
