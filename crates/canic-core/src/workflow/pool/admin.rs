//! Module: workflow::pool::admin
//!
//! Responsibility: route pool admin commands to pool workflow operations.
//! Does not own: endpoint authorization, scheduling, or pool storage mechanics.
//! Boundary: workflow command dispatcher returning admin response DTOs.

use crate::{
    InternalError,
    dto::pool::{PoolAdminCommand, PoolAdminResponse},
    workflow::pool::PoolWorkflow,
};

impl PoolWorkflow {
    pub async fn handle_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, InternalError> {
        match cmd {
            PoolAdminCommand::CreateEmpty(request) => {
                let pid = Self::pool_create_canister(request).await?;
                Ok(PoolAdminResponse::Created { pid })
            }

            PoolAdminCommand::Recycle { pid } => {
                Self::pool_recycle_canister(pid).await?;
                Ok(PoolAdminResponse::Recycled)
            }

            PoolAdminCommand::ImportImmediate { pid } => {
                Self::pool_import_canister(pid).await?;
                Ok(PoolAdminResponse::Imported)
            }

            PoolAdminCommand::ImportQueued { pids } => {
                let result = Self::pool_import_queued_canisters(pids).await?;

                Ok(PoolAdminResponse::QueuedImported { result })
            }
        }
    }
}
