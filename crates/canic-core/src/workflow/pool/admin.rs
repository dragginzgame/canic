//! Pool admin command handling.

use super::PoolWorkflow;
use crate::{
    Error,
    dto::pool::{PoolAdminCommand, PoolAdminResponse},
};

///
/// Entry point for pool admin commands.
///
/// Responsibilities:
/// - Command routing
/// - Response shaping
///
/// Non-responsibilities:
/// - Authorization (handled in workflow / policy)
/// - Scheduling
/// - Pool mechanics
///
impl PoolWorkflow {
    pub async fn handle_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
        match cmd {
            PoolAdminCommand::CreateEmpty => {
                let pid = Self::pool_create_canister().await?;
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
