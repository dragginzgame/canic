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
pub const CANIC_MEMORY_MAX: u8 = 40;

///
/// CANIC stable memory IDs (5–40)
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
    // Topology & discovery state (5–9)
    //
    // Ownership:
    // - Canister topology
    // - App / subnet directories
    //
    // Expected growth: low
    // ---------------------------------------------------------------------

    pub mod topology {
        pub const CANISTER_CHILDREN_ID: u8 = 5;
        pub const APP_DIRECTORY_ID: u8 = 6;
        pub const SUBNET_DIRECTORY_ID: u8 = 7;
        pub const APP_REGISTRY_ID: u8 = 8;
        pub const SUBNET_REGISTRY_ID: u8 = 9;
    }

    // ---------------------------------------------------------------------
    // Environment & configuration state (10)
    //
    // Ownership:
    // - Deployment environment
    // - Static configuration
    //
    // Expected growth: very low
    // ---------------------------------------------------------------------

    pub mod env {
        pub const ENV_ID: u8 = 10;
    }

    // ---------------------------------------------------------------------
    // Auth & signing state (11–14)
    //
    // Ownership:
    // - Delegated signing state
    // - Authorization credentials
    // - Future revocation / rotation metadata
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod auth {
        pub const DELEGATION_STATE_ID: u8 = 11;

        // Reserved: 12–14
    }

    // ---------------------------------------------------------------------
    // Observability & accounting (12–17)
    //
    // Ownership:
    // - Cycles accounting
    // - Logs
    // - Metrics indices
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod observability {
        pub const CYCLE_TRACKER_ID: u8 = 12;
        pub const LOG_INDEX_ID: u8 = 13;
        pub const LOG_DATA_ID: u8 = 14;

        // Reserved: 15–17
    }

    // ---------------------------------------------------------------------
    // Intent & reservation state (18–25)
    //
    // Ownership:
    // - Intent lifecycle (pending → committed / aborted)
    // - Reservation & quota tracking
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod intent {
        pub const INTENT_META_ID: u8 = 18;
        pub const INTENT_RECORDS_ID: u8 = 19;
        pub const INTENT_TOTALS_ID: u8 = 20;
        pub const INTENT_PENDING_ID: u8 = 21;

        // Reserved: 22–25
    }

    // ---------------------------------------------------------------------
    // Pool & capacity state (26–30)
    //
    // Ownership:
    // - Canister pools
    // - Capacity & availability tracking
    //
    // Expected growth: medium
    // ---------------------------------------------------------------------

    pub mod pool {
        pub const CANISTER_POOL_ID: u8 = 26;

        // Reserved: 27–30
    }

    // ---------------------------------------------------------------------
    // Placement, scaling & sharding state (31–36)
    //
    // Ownership:
    // - Placement decisions
    // - Scaling registries
    // - Sharding assignments
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod placement {
        pub const SCALING_REGISTRY_ID: u8 = 31;
        pub const SHARDING_REGISTRY_ID: u8 = 32;
        pub const SHARDING_ASSIGNMENT_ID: u8 = 33;

        // Reserved: 34–36
    }

    // ---------------------------------------------------------------------
    // Application runtime state (37–40)
    //
    // Ownership:
    // - Mutable app / subnet runtime state
    //
    // Expected growth: high
    // ---------------------------------------------------------------------

    pub mod state {
        pub const APP_STATE_ID: u8 = 37;
        pub const SUBNET_STATE_ID: u8 = 38;

        // Reserved: 39–40
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
