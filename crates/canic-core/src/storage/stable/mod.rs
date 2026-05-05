pub mod auth;
pub mod children;
pub mod cycles;
pub mod directory;
pub mod env;
pub mod index;
pub mod intent;
pub mod log;
pub mod pool;
pub mod registry;
pub mod replay;
pub mod scaling;
pub mod security;
#[cfg(feature = "sharding")]
pub mod sharding;
pub mod state;

///
/// CANIC reserves its primary contiguous range during bootstrap.
///

pub const CANIC_MEMORY_MIN: u8 = 13;
pub const CANIC_MEMORY_MAX: u8 = 59;

const _: () = {
    #[canic_memory::__reexports::ctor::ctor(
        unsafe,
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_reserve_topology_memory_range() {
        canic_memory::ic_memory_range!(5, 9);
    }
};

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
    // Topology & discovery state (5–9)
    //
    // Expected growth: low
    // ---------------------------------------------------------------------

    pub mod topology {
        pub const CANISTER_CHILDREN_ID: u8 = 5;
        pub const APP_INDEX_ID: u8 = 6;
        pub const SUBNET_INDEX_ID: u8 = 7;
        pub const APP_REGISTRY_ID: u8 = 8;
        pub const SUBNET_REGISTRY_ID: u8 = 9;

        // 10–12 are owned by `canic-control-plane`.
    }

    // ---------------------------------------------------------------------
    // Environment & configuration state (13–15)
    //
    // Expected growth: very low
    // ---------------------------------------------------------------------

    pub mod env {
        pub const ENV_ID: u8 = 13;
        pub const SUBNET_STATE_ID: u8 = 14;

        // Reserved: 15
    }

    // ---------------------------------------------------------------------
    // Auth & signing state (16–25)
    //
    // Expected growth: medium → high (structural, permanent)
    // ---------------------------------------------------------------------

    pub mod auth {
        pub const AUTH_STATE_ID: u8 = 16;
        pub const ROOT_REPLAY_ID: u8 = 17;

        // Reserved: 18–25
    }

    // ---------------------------------------------------------------------
    // Observability & accounting (26–28)
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod observability {
        pub const CYCLE_TRACKER_ID: u8 = 26;
        pub const LOG_INDEX_ID: u8 = 27;
        pub const LOG_DATA_ID: u8 = 28;

        // Reserved: none
    }

    // ---------------------------------------------------------------------
    // Security events (29–30)
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod security {
        pub const SECURITY_EVENT_INDEX_ID: u8 = 29;
        pub const SECURITY_EVENT_DATA_ID: u8 = 30;

        // Reserved: 31–35
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
        #[cfg(feature = "sharding")]
        pub const SHARDING_REGISTRY_ID: u8 = 50;
        #[cfg(feature = "sharding")]
        pub const SHARDING_ASSIGNMENT_ID: u8 = 51;
        pub const DIRECTORY_REGISTRY_ID: u8 = 52;
        #[cfg(feature = "sharding")]
        pub const SHARDING_ACTIVE_SET_ID: u8 = 53;

        // Reserved for:
        // - placement policies
        // - shard health / liveness
        // - rebalance / drain state
        // - migration metadata
        // 54–58
    }

    // ---------------------------------------------------------------------
    // Application runtime boundary (59)
    //
    // Ownership:
    // - CANIC-controlled runtime state
    // - Upper bound of CANIC ABI
    //
    // Expected growth: low
    //
    // 60 is owned by `canic-control-plane`.
    // ---------------------------------------------------------------------

    pub mod state {
        pub const APP_STATE_ID: u8 = 59;
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

    #[error("security event write failed: current_size={current_size}, delta={delta}")]
    SecurityEventWriteFailed { current_size: u64, delta: u64 },
}

impl From<StableMemoryError> for InternalError {
    fn from(err: StableMemoryError) -> Self {
        StorageError::StableMemory(err).into()
    }
}
