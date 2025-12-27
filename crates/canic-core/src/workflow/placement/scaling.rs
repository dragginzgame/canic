//! Scaling workflow.
//!
//! This module performs scaling side effects:
//! - evaluates scaling policy
//! - creates canisters
//! - mutates the scaling registry
//!
//! All async and IC interactions live here.

use crate::{
    Error, ThisError,
    cdk::utils::time::now_secs,
    dto::{placement::WorkerEntryView, rpc::CreateCanisterParent},
    ops::{rpc::create_canister_request, storage::scaling::ScalingRegistryOps},
    policy::placement::scaling::{ScalingPlan, ScalingPolicy},
};
use candid::Principal;

///
/// ScalingWorkflowError
/// Errors raised during scaling execution
///
#[derive(Debug, ThisError)]
pub enum ScalingWorkflowError {
    #[error("scaling plan rejected: {0}")]
    PlanRejected(String),

    #[error("invalid scaling key: {0}")]
    InvalidKey(String),
}

impl From<ScalingWorkflowError> for Error {
    fn from(err: ScalingWorkflowError) -> Self {
        Self::WorkflowError(err.to_string())
    }
}

///
/// ScalingWorkflow
///
pub struct ScalingWorkflow;

impl ScalingWorkflow {
    /// Create a new worker canister in the given pool, if policy allows.
    pub async fn create_worker(pool: &str) -> Result<Principal, Error> {
        // 1. Evaluate policy
        let ScalingPlan {
            should_spawn,
            reason,
        } = ScalingPolicy::plan_create_worker(pool)?;

        if !should_spawn {
            return Err(ScalingWorkflowError::PlanRejected(reason))?;
        }

        // 2. Look up pool config (policy already validated existence)
        let pool_cfg = {
            let cfg = crate::ops::config::ConfigOps::current_canister();
            cfg.scaling
                .expect("scaling enabled by policy")
                .pools
                .get(pool)
                .expect("pool validated by policy")
                .clone()
        };

        let role = pool_cfg.canister_role.clone();

        // 3. Create the canister
        let pid = create_canister_request::<()>(&role, CreateCanisterParent::ThisCanister, None)
            .await?
            .new_canister_pid;

        // 4. Register in memory
        let entry = WorkerEntryView::try_new(pool, role, now_secs())
            .map_err(ScalingWorkflowError::InvalidKey)?;

        ScalingRegistryOps::insert(pid, entry);

        Ok(pid)
    }
}
