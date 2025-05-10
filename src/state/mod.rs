pub mod app_state;
pub mod canister_state;
pub mod child_index;
pub mod subnet_index;

pub use app_state::{AppCommand, AppMode, AppState, AppStateData};
pub use canister_state::{CanisterState, CanisterStateData, CanisterStateError};
pub use child_index::{ChildIndex, ChildIndexData, ChildIndexError};
pub use subnet_index::{SubnetIndex, SubnetIndexData, SubnetIndexError};

use crate::{ic::structures::memory::MemoryId, memory_manager};
use std::cell::RefCell;

//
// MEMORY_MANAGER
//

memory_manager!();

///
/// CORE STATE
/// every canister implements these
///
/// AppState and SubnetIndex live on root, and can be cached on other canisters
/// Every canister has its own CanisterState
///

// global memory ids are hardcoded
const APP_STATE_MEMORY_ID: u8 = 1;
const SUBNET_INDEX_MEMORY_ID: u8 = 2;
const CANISTER_STATE_MEMORY_ID: u8 = 3;
const CHILD_INDEX_MEMORY_ID: u8 = 4;

thread_local! {

    ///
    /// APP_STATE
    ///
    /// Scope     : Application
    /// Structure : Cell
    ///
    /// a Cell that's only really meant for small data structures used for global app state
    ///
    /// defaults to Enabled as then it's possible for non-controllers to call
    /// endpoints in order to initialise
    ///

    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(APP_STATE_MEMORY_ID))),
        AppMode::Enabled,
    ));

    ///
    /// CANISTER_STATE
    ///
    /// Scope     : Canister
    /// Structure : Cell
    ///

    pub static CANISTER_STATE: RefCell<CanisterState> = RefCell::new(CanisterState::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(CANISTER_STATE_MEMORY_ID))),
    ));

    ///
    /// CHILD_INDEX
    ///
    /// Scope     : Canister
    /// Structure : BTreeMap
    ///

    pub static CHILD_INDEX: RefCell<ChildIndex> = RefCell::new(ChildIndex::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(CHILD_INDEX_MEMORY_ID))),
    ));

    ///
    /// SUBNET_INDEX
    ///
    /// Scope     : Subnet
    /// Structure : BTreeMap
    ///

    pub static SUBNET_INDEX: RefCell<SubnetIndex> = RefCell::new(SubnetIndex::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(SUBNET_INDEX_MEMORY_ID))),
    ));

}
