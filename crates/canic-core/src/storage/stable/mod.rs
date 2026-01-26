pub mod auth;
pub mod children;
pub mod cycles;
pub mod directory;
pub mod env;
pub mod intent;
pub mod log;
pub mod pool;
pub mod registry;
pub mod scaling;
pub mod sharding;
pub mod state;

///
/// CANIC is only allowed to allocate within this inclusive range.
///

pub const CANIC_MEMORY_MIN: u8 = 5;
pub const CANIC_MEMORY_MAX: u8 = 60;

///
/// CANIC stable memory IDs
///
/// ⚠️ PRE-FREEZE LAYOUT
/// IDs may change until this layout is finalized.
/// Once frozen, IDs are ABI-stable and MUST NOT be renumbered or reused.
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
    // Topology & discovery state (5–12)
    //
    // Expected growth: low
    // ---------------------------------------------------------------------

    pub mod topology {
        pub const CANISTER_CHILDREN_ID: u8 = 5;
        pub const APP_DIRECTORY_ID: u8 = 6;
        pub const SUBNET_DIRECTORY_ID: u8 = 7;
        pub const APP_REGISTRY_ID: u8 = 8;
        pub const SUBNET_REGISTRY_ID: u8 = 9;

        // Reserved: 10–12
    }

    // ---------------------------------------------------------------------
    // Environment & configuration state (13–15)
    //
    // Expected growth: very low
    // ---------------------------------------------------------------------

    pub mod env {
        pub const ENV_ID: u8 = 13;

        // Reserved: 14–15
    }

    // ---------------------------------------------------------------------
    // Auth & signing state (16–25)
    //
    // Expected growth: medium → high (structural, permanent)
    // ---------------------------------------------------------------------

    pub mod auth {
        pub const DELEGATION_STATE_ID: u8 = 16;

        // Reserved: 17–25
    }

    // ---------------------------------------------------------------------
    // Observability & accounting (26–35)
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod observability {
        pub const CYCLE_TRACKER_ID: u8 = 26;
        pub const LOG_INDEX_ID: u8 = 27;
        pub const LOG_DATA_ID: u8 = 28;

        // Reserved: 29–35
    }

    // ---------------------------------------------------------------------
    // Intent & reservation state (36–45)
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod intent {
        pub const INTENT_META_ID: u8 = 36;
        pub const INTENT_RECORDS_ID: u8 = 37;
        pub const INTENT_TOTALS_ID: u8 = 38;
        pub const INTENT_PENDING_ID: u8 = 39;

        // Reserved: 40–45
    }

    // ---------------------------------------------------------------------
    // Pool & capacity state (46–48)
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod pool {
        pub const CANISTER_POOL_ID: u8 = 46;

        // Reserved: 47–48
    }

    // ---------------------------------------------------------------------
    // Placement, scaling & sharding state (49–58)
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod placement {
        pub const SCALING_REGISTRY_ID: u8 = 49;
        pub const SHARDING_REGISTRY_ID: u8 = 50;
        pub const SHARDING_ASSIGNMENT_ID: u8 = 51;
        pub const SHARDING_LIFECYCLE_ID: u8 = 52;
        pub const SHARDING_ACTIVE_SET_ID: u8 = 53;
        pub const SHARDING_ROTATION_TARGETS_ID: u8 = 54;

        // Reserved for:
        // - placement policies
        // - shard health / liveness
        // - rebalance / drain state
        // - migration metadata
        // 55–58
    }

    // ---------------------------------------------------------------------
    // Application / subnet runtime boundary (59–60)
    //
    // Ownership:
    // - CANIC-controlled runtime state
    // - Upper bound of CANIC ABI
    //
    // Expected growth: low
    // ---------------------------------------------------------------------

    pub mod state {
        pub const APP_STATE_ID: u8 = 59;
        pub const SUBNET_STATE_ID: u8 = 60;
    }
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
