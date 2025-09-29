pub mod canister;
pub mod registry;
pub mod root;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod subnet;
pub mod types;

pub use registry::{MemoryRegistry, MemoryRegistryError};
pub use types::*;

use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::MemoryManager},
    memory::{
        canister::CanisterRootError,
        scaling::ScalingRegistryError,
        sharding::ShardingRegistryError,
        state::{AppStateError, CanisterStateError},
        subnet::SubnetError,
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
/// ICU is only allowed to allocate within this inclusive range.
/// Keep small but with room for future expansion.
///

pub(crate) const ICU_MEMORY_MIN: u8 = 5;
pub(crate) const ICU_MEMORY_MAX: u8 = 30;

///
/// ICU Memory IDs (5-30)
///

pub(crate) mod id {
    // icu network states
    // should remain just three, app -> subnet -> canister
    pub mod state {
        pub const APP_STATE_ID: u8 = 5;
        pub const SUBNET_STATE_ID: u8 = 6;
        pub const CANISTER_STATE_ID: u8 = 7;
    }

    // subnet
    // registry is root-authoritative, the others are cascaded views
    pub mod subnet {
        pub const SUBNET_REGISTRY_ID: u8 = 8;
        pub const SUBNET_CHILDREN_ID: u8 = 9;
        pub const SUBNET_DIRECTORY_ID: u8 = 10;
        pub const SUBNET_PARENTS_ID: u8 = 11;
    }

    // root
    // various structures handled solely by root
    pub mod root {
        pub const CANISTER_RESERVE_ID: u8 = 15;
    }

    // canister
    // every canister has these structures
    pub mod canister {
        pub const CANISTER_ROOT_ID: u8 = 18;
        pub const CYCLE_TRACKER_ID: u8 = 19;
    }

    // capability
    // canisters can optionally have these
    pub mod capability {
        pub const SCALING_REGISTRY_ID: u8 = 24;
        pub const SHARDING_REGISTRY_ID: u8 = 25;
        pub const SHARDING_TENANTS_ID: u8 = 26;
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
    AppStateError(#[from] AppStateError),

    #[error(transparent)]
    CanisterRootError(#[from] CanisterRootError),

    #[error(transparent)]
    CanisterStateError(#[from] CanisterStateError),

    #[error(transparent)]
    ScalingRegistryError(#[from] ScalingRegistryError),

    #[error(transparent)]
    ShardingRegistryError(#[from] ShardingRegistryError),

    #[error(transparent)]
    SubnetError(#[from] SubnetError),
}
