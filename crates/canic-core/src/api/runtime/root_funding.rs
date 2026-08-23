pub use crate::ops::runtime::root_funding::{
    RootFundingRuntime, RootFundingRuntimeApi, RootFundingRuntimeConfig,
};

/// Lifecycle-facing control over the sole Root cycle-top-up owner.
pub struct RootFundingTimerApi;

impl RootFundingTimerApi {
    /// Re-read protected demand and reconcile the existing native timer claim.
    pub fn reconcile() -> Result<(), crate::dto::error::Error> {
        crate::workflow::runtime::cycles::CycleWorkflow::start().map_err(Into::into)
    }

    /// Disarm the sole timer only when no funding or refill attempt is executing.
    pub fn prepare_policy_rotation_fence() -> Result<(), crate::error::InternalError> {
        use crate::{
            ops::storage::{async_job_recovery::AsyncJobOwner, icp_refill::IcpRefillStoreOps},
            workflow::runtime::async_job::AsyncJobWorkflow,
        };

        if AsyncJobWorkflow::has_active_attempt(AsyncJobOwner::CycleTopup)
            || IcpRefillStoreOps::resumable_operation_count() != 0
        {
            return Err(crate::error::InternalError::conflict());
        }
        crate::workflow::runtime::cycles::CycleWorkflow::cancel_timer()?;
        Ok(())
    }

    /// Durably disable and disarm new Root funding before terminal deletion work.
    pub fn fence_for_deletion() -> Result<(), crate::error::InternalError> {
        use crate::{
            ops::storage::{
                async_job_recovery::AsyncJobOwner,
                icp_refill::IcpRefillStoreOps,
                state::fleet::{FleetStateCommand, FleetStateOps},
            },
            workflow::runtime::async_job::AsyncJobWorkflow,
        };

        if AsyncJobWorkflow::has_active_attempt(AsyncJobOwner::CycleTopup)
            || IcpRefillStoreOps::resumable_operation_count() != 0
        {
            return Err(crate::error::InternalError::conflict());
        }
        let _ = FleetStateOps::execute_command(FleetStateCommand::SetCyclesFundingEnabled(false));
        crate::workflow::runtime::cycles::CycleWorkflow::cancel_timer()?;
        Ok(())
    }
}
