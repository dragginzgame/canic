pub mod directory;
pub mod env;
pub mod ext;
pub mod registry;
pub mod root;
pub mod state;
pub mod topology;
pub mod types;

pub use env::Env;
pub use registry::{MemoryRegistry, MemoryRegistryError};
pub use types::*;

use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::MemoryManager},
    memory::{
        directory::DirectoryError, env::ContextError, ext::ExtensionError, state::StateError,
        topology::TopologyError,
    },
};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// Reserved for the registry system itself
///

pub(crate) const MEMORY_REGISTRY_ID: u8 = 0;
pub(crate) const MEMORY_RANGES_ID: u8 = 1;

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

    // icu network states
    pub mod state {
        pub const APP_STATE_ID: u8 = 7;
        pub const SUBNET_STATE_ID: u8 = 8;
    }

    // directory
    pub mod directory {
        pub const SUBNET_DIRECTORY_ID: u8 = 10;
        pub const APP_DIRECTORY_ID: u8 = 11;
    }

    // topology
    pub mod topology {
        pub mod app {
            // prime root is authoritative
            pub const APP_SUBNET_REGISTRY_ID: u8 = 13;
        }

        pub mod subnet {
            // registry is root-authoritative, the others are cascaded views
            pub const SUBNET_CANISTER_REGISTRY_ID: u8 = 15;
            pub const SUBNET_CANISTER_CHILDREN_ID: u8 = 17;
            pub const SUBNET_CANISTER_PARENTS_ID: u8 = 18;
        }
    }

    // root
    // various structures handled solely by root
    pub mod root {
        pub const CANISTER_RESERVE_ID: u8 = 20;
    }

    // capability
    pub mod capability {
        pub mod cycles {
            pub const CYCLE_TRACKER_ID: u8 = 24;
        }

        pub mod scaling {
            pub const SCALING_REGISTRY_ID: u8 = 25;
        }

        pub mod sharding {
            pub const SHARDING_REGISTRY_ID: u8 = 26;
            pub const SHARDING_TENANTS_ID: u8 = 27;
        }
    }
}

//
// MEMORY_MANAGER
//

thread_local! {

    ///
    /// Define MEMORY_MANAGER thread-locally for the entire scope
    ///

    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(
            DefaultMemoryImpl::default()
        ));
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
    ContextError(#[from] ContextError),

    #[error(transparent)]
    DirectoryError(#[from] DirectoryError),

    #[error(transparent)]
    ExtensionError(#[from] ExtensionError),

    #[error(transparent)]
    StateError(#[from] StateError),

    #[error(transparent)]
    TopologyError(#[from] TopologyError),
}
