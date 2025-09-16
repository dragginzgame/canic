use crate::{
    Error,
    memory::ShardRegistry,
    ops::{
        prelude::*,
        shard::{decommission_shard, drain_shard, rebalance_pool},
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// AdminCommand
///
/// Administrative shard operations, combined under a single endpoint.
///

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub enum AdminCommand {
    Register {
        pid: Principal,
        pool: String,
        capacity: u32,
    },
    Repair,
    Drain {
        pool: String,
        shard_pid: Principal,
        max_moves: u32,
    },
    Rebalance {
        pool: String,
        max_moves: u32,
    },
    Decommission {
        shard_pid: Principal,
    },
}

///
/// AdminResult
///

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub enum AdminResult {
    Ok,
    Moved(u32),
}

/// admin
/// Run a shard admin command.
pub async fn admin_command(cmd: AdminCommand) -> Result<AdminResult, Error> {
    match cmd {
        AdminCommand::Register {
            pid,
            pool,
            capacity,
        } => {
            ShardRegistry::register(pid, &pool, capacity);

            Ok(AdminResult::Ok)
        }

        AdminCommand::Repair => {
            ShardRegistry::repair_counts();

            Ok(AdminResult::Ok)
        }

        AdminCommand::Drain {
            pool,
            shard_pid,
            max_moves,
        } => {
            let moved = drain_shard(&pool, shard_pid, max_moves).await?;

            Ok(AdminResult::Moved(moved))
        }

        AdminCommand::Rebalance { pool, max_moves } => {
            let moved = rebalance_pool(&pool, max_moves)?;
            Ok(AdminResult::Moved(moved))
        }

        AdminCommand::Decommission { shard_pid } => {
            decommission_shard(shard_pid)?;
            Ok(AdminResult::Ok)
        }
    }
}
