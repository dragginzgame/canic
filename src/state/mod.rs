pub mod app_state;
pub mod canister_state;
pub mod child_index;
pub mod subnet_index;

pub use app_state::{APP_STATE, AppCommand, AppMode, AppState, AppStateData};
pub use canister_state::{CANISTER_STATE, CanisterState, CanisterStateData, CanisterStateError};
pub use child_index::{CHILD_INDEX, ChildIndex, ChildIndexData, ChildIndexError};
pub use subnet_index::{SUBNET_INDEX, SubnetIndex, SubnetIndexData, SubnetIndexError};

use crate::memory_manager;

//
// MEMORY_MANAGER
//

memory_manager!();

// global memory ids are hardcoded
const APP_STATE_MEMORY_ID: u8 = 1;
const SUBNET_INDEX_MEMORY_ID: u8 = 2;
const CANISTER_STATE_MEMORY_ID: u8 = 3;
const CHILD_INDEX_MEMORY_ID: u8 = 4;
