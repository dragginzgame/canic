pub mod app_state;
pub mod canister_state;
pub mod child_index;
pub mod subnet_index;

pub use app_state::{AppCommand, AppMode, AppState, AppStateData, AppStateError};
pub use canister_state::{CanisterState, CanisterStateData, CanisterStateError};
pub use child_index::{ChildIndex, ChildIndexData, ChildIndexError};
pub use subnet_index::{SubnetIndex, SubnetIndexData, SubnetIndexError};

use crate::memory::allocate_state;
use std::cell::RefCell;

thread_local! {

    pub static APP_STATE: RefCell<AppState> = RefCell::new(
        allocate_state(|mem| AppState::init(mem, AppMode::Enabled))
    );

    pub static CHILD_INDEX: RefCell<ChildIndex> = RefCell::new(
        allocate_state(ChildIndex::init)
    );

    pub static CANISTER_STATE: RefCell<CanisterState> = RefCell::new(
        allocate_state(CanisterState::init)
    );

    pub static SUBNET_INDEX: RefCell<SubnetIndex> = RefCell::new(
        allocate_state(SubnetIndex::init)
    );

}
