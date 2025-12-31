pub mod children;
pub mod cycles;
pub mod directory;
pub mod env;
pub mod log;
pub mod pool;
pub mod registry;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod types;

///
/// CANIC is only allowed to allocate within this inclusive range.
/// Keep small but with room for future expansion.
///

pub const CANIC_MEMORY_MIN: u8 = 5;
pub const CANIC_MEMORY_MAX: u8 = 30;

///
/// CANIC Memory IDs (5-30)
///

pub mod id {
    pub mod children {
        pub const CANISTER_CHILDREN_ID: u8 = 5;
    }

    pub mod cycles {
        pub const CYCLE_TRACKER_ID: u8 = 7;
    }

    pub mod directory {
        pub const APP_DIRECTORY_ID: u8 = 9;
        pub const SUBNET_DIRECTORY_ID: u8 = 10;
    }

    pub mod env {
        pub const ENV_ID: u8 = 12;
    }

    pub mod log {
        pub const LOG_INDEX_ID: u8 = 14;
        pub const LOG_DATA_ID: u8 = 15;
    }

    pub mod pool {
        pub const CANISTER_POOL_ID: u8 = 17;
    }

    pub mod registry {
        pub const APP_REGISTRY_ID: u8 = 19;
        pub const SUBNET_REGISTRY_ID: u8 = 20;
    }

    pub mod scaling {
        pub const SCALING_REGISTRY_ID: u8 = 22;
    }

    pub mod sharding {
        pub const SHARDING_REGISTRY_ID: u8 = 24;
        pub const SHARDING_ASSIGNMENT_ID: u8 = 25;
    }

    pub mod state {
        pub const APP_STATE_ID: u8 = 27;
        pub const SUBNET_STATE_ID: u8 = 28;
    }
}

use crate::{Error, model::ModelError};
use thiserror::Error as ThisError;

///
/// MemoryError
///

#[derive(Debug, ThisError)]
pub enum MemoryError {
    #[error("log write failed: current_size={current_size}, delta={delta}")]
    LogWriteFailed { current_size: u64, delta: u64 },
}

impl From<MemoryError> for Error {
    fn from(err: MemoryError) -> Self {
        ModelError::Memory(err).into()
    }
}
