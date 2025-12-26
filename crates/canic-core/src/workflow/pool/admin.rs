//! Pool admin command handling.
//!
//! This module is the *boundary-facing* side of the pool workflow.
//! It translates admin commands into workflow actions and assembles
//! structured responses.
//!
//! It does NOT:
//! - perform scheduling
//! - contain policy decisions
//! - implement pool mechanics
//!
//! Those responsibilities live in:
//! - scheduler.rs (timers / workers)
//! - policy::pool (decisions)
//! - workflow.rs (orchestration)

use crate::{
    Error,
    ops::OpsError,
    ops::ic::{Network, build_network},
    workflow::pool::{
        dto::{PoolAdminCommand, PoolAdminResponse},
        workflow::{
            pool_create_canister, pool_import_canister, pool_import_queued_canisters,
            pool_import_queued_canisters_local, pool_recycle_canister, pool_requeue_failed,
        },
    },
};

/// Entry point for pool admin commands.
///
/// Authorization:
/// - Root-only (enforced here to keep endpoints thin)
///
/// Semantics:
/// - This function performs *routing*, not orchestration.
/// - Each command delegates to an appropriate workflow function.
pub async fn handle_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
    // All pool admin operations are root-only
    OpsError::require_root()?;

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
                if build_network() == Some(Network::Local) {
                    pool_import_queued_canisters_local(pids).await?
                } else {
                    pool_import_queued_canisters(pids)?
                };

            Ok(PoolAdminResponse::QueuedImported {
                added,
                requeued,
                skipped,
                total,
                summary,
            })
        }

        PoolAdminCommand::RequeueFailed { pids } => {
            let (requeued, skipped, total) = pool_requeue_failed(pids)?;
            Ok(PoolAdminResponse::FailedRequeued {
                requeued,
                skipped,
                total,
            })
        }
    }
}
