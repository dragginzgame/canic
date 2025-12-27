//! Pool admin command handling.

use crate::{
    Error,
    workflow::pool::{
        dto::{PoolAdminCommand, PoolAdminResponse},
        pool_create_canister, pool_import_canister, pool_import_queued_canisters,
        pool_recycle_canister,
    },
};

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
pub async fn handle_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
    match cmd {
        PoolAdminCommand::CreateEmpty => {
            let pid = pool_create_canister().await?;
            Ok(PoolAdminResponse::Created { pid })
        }

        PoolAdminCommand::Recycle { pid } => {
            pool_recycle_canister(pid).await?;
            Ok(PoolAdminResponse::Recycled)
        }

        PoolAdminCommand::ImportImmediate { pid } => {
            pool_import_canister(pid).await?;
            Ok(PoolAdminResponse::Imported)
        }

        PoolAdminCommand::ImportQueued { pids } => {
            let (added, requeued, skipped, total, summary) =
                pool_import_queued_canisters(pids).await?;

            Ok(PoolAdminResponse::QueuedImported {
                added,
                requeued,
                skipped,
                total,
                summary,
            })
        }
    }
}
