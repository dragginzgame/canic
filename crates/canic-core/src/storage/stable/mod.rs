pub mod auth;
pub mod children;
pub mod cycles;
pub mod directory;
pub mod env;
pub mod icp_refill;
pub mod index;
pub mod intent;
pub mod log;
pub mod pool;
pub mod registry;
pub mod replay;
pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
pub mod state;

///
/// CANIC reserves its primary contiguous range during bootstrap.
///

pub const CANIC_MEMORY_MIN: u8 = 11;
pub const CANIC_MEMORY_MAX: u8 = 79;

///
/// CANIC stable memory IDs
///
/// Principles:
/// - IDs are grouped by state authority
/// - Each group owns a contiguous block
/// - Gaps between blocks are intentional expansion reserves
///

pub mod memory {

    // =====================================================================
    // Stable memory layout
    //
    // Conventions:
    // - IDs are permanent once assigned
    // - Ranges are intentionally reserved for future growth
    // - Modules own their entire numeric range
    // - This file is ordered by increasing ID, not by dependency
    // =====================================================================

    // ---------------------------------------------------------------------
    // Topology & discovery state (11–15)
    //
    // Expected growth: low
    // ---------------------------------------------------------------------

    pub mod topology {
        pub const CANISTER_CHILDREN_ID: u8 = 11;
        pub const APP_INDEX_ID: u8 = 12;
        pub const SUBNET_INDEX_ID: u8 = 13;
        pub const APP_REGISTRY_ID: u8 = 14;
        pub const SUBNET_REGISTRY_ID: u8 = 15;
    }

    // ---------------------------------------------------------------------
    // Environment & runtime state (16–18)
    //
    // Expected growth: very low
    // ---------------------------------------------------------------------

    pub mod env {
        pub const ENV_ID: u8 = 16;
        pub const SUBNET_STATE_ID: u8 = 17;
        pub const APP_STATE_ID: u8 = 18;
    }

    // ---------------------------------------------------------------------
    // Auth & signing state (19–28)
    //
    // Expected growth: medium → high (structural, permanent)
    // ---------------------------------------------------------------------

    pub mod auth {
        pub const AUTH_STATE_ID: u8 = 19;
        #[allow(dead_code)]
        // Historical root replay memory ID. Root replay moved to REPLAY_RECEIPTS_ID.
        pub const ROOT_REPLAY_ID: u8 = 20;
        pub const REPLAY_RECEIPTS_ID: u8 = 21;

        // Reserved: 22–28
    }

    // ---------------------------------------------------------------------
    // Observability & accounting (29–38)
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod observability {
        pub const CYCLE_TRACKER_ID: u8 = 29;
        pub const CYCLE_TOPUP_EVENTS_ID: u8 = 30;
        pub const LOG_INDEX_ID: u8 = 31;
        pub const LOG_DATA_ID: u8 = 32;
        pub const ICP_REFILL_RECORDS_ID: u8 = 33;

        // Reserved: 34–38
    }

    // ---------------------------------------------------------------------
    // Intent & reservation state (39–48)
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod intent {
        pub const INTENT_META_ID: u8 = 39;
        pub const INTENT_RECORDS_ID: u8 = 40;
        pub const INTENT_TOTALS_ID: u8 = 41;
        pub const INTENT_PENDING_ID: u8 = 42;

        // Reserved: 43–48
    }

    // ---------------------------------------------------------------------
    // Pool & capacity state (49–51)
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod pool {
        pub const CANISTER_POOL_ID: u8 = 49;

        // Reserved: 50–51
    }

    // ---------------------------------------------------------------------
    // Placement, scaling & sharding state (52–61)
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod placement {
        pub const SCALING_REGISTRY_ID: u8 = 52;
        #[cfg(feature = "sharding")]
        pub const SHARDING_REGISTRY_ID: u8 = 53;
        #[cfg(feature = "sharding")]
        pub const SHARDING_ASSIGNMENT_ID: u8 = 54;
        pub const DIRECTORY_REGISTRY_ID: u8 = 55;
        #[cfg(feature = "sharding")]
        pub const SHARDING_ACTIVE_SET_ID: u8 = 56;

        // Reserved for:
        // - placement policies
        // - shard health / liveness
        // - rebalance / drain state
        // - migration metadata
        // 57–61
    }

    // 62-79 remain long-horizon Canic core reserve.
}

use crate::{InternalError, storage::prelude::*};
use thiserror::Error as ThisError;

///
/// StableMemoryError
///

#[derive(Debug, ThisError)]
pub enum StableMemoryError {
    #[error("log write failed: current_size={current_size}, delta={delta}")]
    LogWriteFailed { current_size: u64, delta: u64 },
}

impl From<StableMemoryError> for InternalError {
    fn from(err: StableMemoryError) -> Self {
        StorageError::StableMemory(err).into()
    }
}
