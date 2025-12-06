pub mod cycles;
pub mod directory;
pub mod env;
pub mod log;
pub mod reserve;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod topology;
pub mod types;

pub use canic_memory::MemoryRegistryError;
pub(crate) use env::Env;
pub use types::*;

use crate::{
    Error,
    model::{ModelError, memory::log::LogError},
};
use thiserror::Error as ThisError;

///
/// CANIC is only allowed to allocate within this inclusive range.
/// Keep small but with room for future expansion.
///

pub(crate) const CANIC_MEMORY_MIN: u8 = 5;
pub(crate) const CANIC_MEMORY_MAX: u8 = 30;

///
/// CANIC Memory IDs (5-30)
///

pub(crate) mod id {
    // environment
    // creation-only, and it stays immutable
    // all canisters get env
    pub const ENV_ID: u8 = 5;

    // subnet-level state payloads
    pub mod state {
        pub const APP_STATE_ID: u8 = 7;
        pub const SUBNET_STATE_ID: u8 = 8;
    }

    // directory
    pub mod directory {
        pub const APP_DIRECTORY_ID: u8 = 10;
        pub const SUBNET_DIRECTORY_ID: u8 = 11;
    }

    // log
    pub mod log {
        pub const LOG_INDEX_ID: u8 = 13;
        pub const LOG_DATA_ID: u8 = 14;
    }

    // topology
    pub mod topology {
        pub mod app {
            // prime root is authoritative
            pub const APP_SUBNET_REGISTRY_ID: u8 = 16;
        }

        pub mod subnet {
            // registry is root-authoritative, the others are cascaded views
            pub const SUBNET_CANISTER_REGISTRY_ID: u8 = 17;
            pub const SUBNET_CANISTER_CHILDREN_ID: u8 = 18;
        }
    }

    // root
    // various structures handled solely by root
    pub mod root {
        pub const CANISTER_RESERVE_ID: u8 = 20;
    }

    // cycles
    pub mod cycles {
        pub const CYCLE_TRACKER_ID: u8 = 24;
    }

    // scaling
    pub mod scaling {
        pub const SCALING_REGISTRY_ID: u8 = 26;
    }

    // sharding
    pub mod sharding {
        pub const SHARDING_REGISTRY_ID: u8 = 27;
        pub const SHARDING_ASSIGNMENT_ID: u8 = 28;
    }
}

///
/// MemoryError
///

#[derive(Debug, ThisError)]
pub enum MemoryError {
    // top level registry error
    #[error(transparent)]
    MemoryRegistryError(#[from] MemoryRegistryError),

    #[error(transparent)]
    LogError(#[from] LogError),
}

impl From<MemoryError> for Error {
    fn from(err: MemoryError) -> Self {
        ModelError::MemoryError(err).into()
    }
}
