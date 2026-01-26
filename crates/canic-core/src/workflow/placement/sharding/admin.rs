//! Root-only sharding lifecycle control-plane commands.

use super::ShardingWorkflow;
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::placement::sharding::{ShardingAdminCommand, ShardingAdminResponse},
    ids::ShardLifecycleState,
    ops::storage::placement::sharding_lifecycle::ShardingLifecycleOps,
};

///
/// Sharding admin command handling.
///

impl ShardingWorkflow {
    #[allow(clippy::unused_async)]
    pub async fn handle_admin(
        cmd: ShardingAdminCommand,
    ) -> Result<ShardingAdminResponse, InternalError> {
        match cmd {
            ShardingAdminCommand::RegisterShardCreated { pid } => {
                Self::register_shard_created(pid)?;
                Ok(ShardingAdminResponse::Registered { pid })
            }
            ShardingAdminCommand::MarkShardProvisioned { pid } => {
                Self::mark_shard_provisioned(pid)?;
                Ok(ShardingAdminResponse::Provisioned { pid })
            }
            ShardingAdminCommand::AdmitShardToHrw { pid } => {
                Self::admit_shard_to_hrw(pid)?;
                Ok(ShardingAdminResponse::Admitted { pid })
            }
            ShardingAdminCommand::RetireShard { pid } => {
                Self::retire_shard(pid)?;
                Ok(ShardingAdminResponse::Retired { pid })
            }
            ShardingAdminCommand::RevokeShard { pid } => {
                Self::revoke_shard(pid)?;
                Ok(ShardingAdminResponse::Revoked { pid })
            }
        }
    }

    pub(crate) fn register_shard_created(pid: Principal) -> Result<(), InternalError> {
        match ShardingLifecycleOps::state(pid) {
            None => {
                ShardingLifecycleOps::set_state(pid, ShardLifecycleState::Created);
                Ok(())
            }
            Some(ShardLifecycleState::Created) => Ok(()),
            Some(state) => Err(invalid_transition(
                pid,
                Some(state),
                ShardLifecycleState::Created,
            )),
        }
    }

    pub(crate) fn mark_shard_provisioned(pid: Principal) -> Result<(), InternalError> {
        match ShardingLifecycleOps::state(pid) {
            Some(ShardLifecycleState::Created) => {
                ShardingLifecycleOps::set_state(pid, ShardLifecycleState::Provisioned);
                Ok(())
            }
            Some(ShardLifecycleState::Provisioned) => Ok(()),
            Some(state) => Err(invalid_transition(
                pid,
                Some(state),
                ShardLifecycleState::Provisioned,
            )),
            None => Err(not_registered(pid)),
        }
    }

    pub(crate) fn admit_shard_to_hrw(pid: Principal) -> Result<(), InternalError> {
        match ShardingLifecycleOps::state(pid) {
            Some(ShardLifecycleState::Provisioned) => {
                ShardingLifecycleOps::set_state(pid, ShardLifecycleState::Active);
                ShardingLifecycleOps::set_active(pid);
                ShardingLifecycleOps::set_rotation_target(pid);
                Ok(())
            }
            Some(ShardLifecycleState::Active) => {
                ShardingLifecycleOps::set_active(pid);
                ShardingLifecycleOps::set_rotation_target(pid);
                Ok(())
            }
            Some(state) => Err(invalid_transition(
                pid,
                Some(state),
                ShardLifecycleState::Active,
            )),
            None => Err(not_registered(pid)),
        }
    }

    pub(crate) fn retire_shard(pid: Principal) -> Result<(), InternalError> {
        match ShardingLifecycleOps::state(pid) {
            Some(ShardLifecycleState::Active) => {
                ShardingLifecycleOps::set_state(pid, ShardLifecycleState::Retiring);
                ShardingLifecycleOps::clear_active(pid);
                ShardingLifecycleOps::clear_rotation_target(pid);
                Ok(())
            }
            Some(ShardLifecycleState::Retiring) => {
                ShardingLifecycleOps::clear_active(pid);
                ShardingLifecycleOps::clear_rotation_target(pid);
                Ok(())
            }
            Some(state) => Err(invalid_transition(
                pid,
                Some(state),
                ShardLifecycleState::Retiring,
            )),
            None => Err(not_registered(pid)),
        }
    }

    pub(crate) fn revoke_shard(pid: Principal) -> Result<(), InternalError> {
        match ShardingLifecycleOps::state(pid) {
            Some(ShardLifecycleState::Revoked) => {
                ShardingLifecycleOps::clear_active(pid);
                ShardingLifecycleOps::clear_rotation_target(pid);
                Ok(())
            }
            Some(_) => {
                ShardingLifecycleOps::set_state(pid, ShardLifecycleState::Revoked);
                ShardingLifecycleOps::clear_active(pid);
                ShardingLifecycleOps::clear_rotation_target(pid);
                Ok(())
            }
            None => Err(not_registered(pid)),
        }
    }
}

fn invalid_transition(
    pid: Principal,
    from: Option<ShardLifecycleState>,
    to: ShardLifecycleState,
) -> InternalError {
    InternalError::invariant(
        InternalErrorOrigin::Workflow,
        format!("invalid shard lifecycle transition: pid={pid} from={from:?} to={to:?}"),
    )
}

fn not_registered(pid: Principal) -> InternalError {
    InternalError::invariant(
        InternalErrorOrigin::Workflow,
        format!("shard lifecycle state missing for pid={pid}"),
    )
}
