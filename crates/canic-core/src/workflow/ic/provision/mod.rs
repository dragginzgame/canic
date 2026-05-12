// =============================================================================
// PROVISIONING (ROOT ORCHESTRATOR HELPERS)
// =============================================================================

//! Provisioning helpers for creating, installing, and tearing down canisters.
//!
//! These routines bundle the multi-phase orchestration that root performs when
//! scaling out the topology: reserving cycles, recording registry state,
//! installing WASM modules, and cascading state updates to descendants.

mod allocation;
mod delete;
mod indexes;
mod install;
mod metrics;
mod payload;
mod policy;

use crate::{
    InternalError, InternalErrorOrigin,
    api::runtime::install::ModuleSourceRuntimeApi,
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

use crate::ops::runtime::metrics::{
    canister_ops::{CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason},
    provisioning::{
        ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
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

        let (pid, source) = allocate_canister(role, parent_pid).await?;

        if let Err(err) = install_canister(pid, role, parent_pid, &module_source, extra_arg).await {
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
