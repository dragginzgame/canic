use crate::{
    ic::structures::{Cell, DefaultMemory, cell::CellError, memory::MemoryId},
    impl_storable_unbounded,
    state::{CANISTER_STATE_MEMORY_ID, MEMORY_MANAGER},
};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

///
/// CanisterStateError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum CanisterStateError {
    #[error("path has not been set")]
    PathNotSet,

    #[error("root_id has not been set")]
    RootIdNotSet,

    #[error(transparent)]
    CellError(#[from] CellError),
}

//
// CANISTER_STATE
//

thread_local! {
    pub static CANISTER_STATE: RefCell<CanisterState> = RefCell::new(CanisterState::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(CANISTER_STATE_MEMORY_ID))),
    ));
}

///
/// CanisterState
///

#[derive(Deref, DerefMut)]
pub struct CanisterState(Cell<CanisterStateData>);

impl CanisterState {
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        Self(Cell::init(memory, CanisterStateData::default()).unwrap())
    }

    // get_data
    #[must_use]
    pub fn get_data(&self) -> CanisterStateData {
        self.get()
    }

    // set_data
    pub fn set_data(&mut self, data: CanisterStateData) -> Result<(), CanisterStateError> {
        self.set(data)?;

        Ok(())
    }

    // get_path
    pub fn get_path(&self) -> Result<String, CanisterStateError> {
        self.get().path.ok_or(CanisterStateError::PathNotSet)
    }

    // set_path
    pub fn set_path(&mut self, path: &str) -> Result<(), CanisterStateError> {
        let mut state = self.get();
        state.path = Some(path.to_string());
        self.set(state)?;

        Ok(())
    }

    // get_root_pid
    pub fn get_root_pid(&self) -> Result<Principal, CanisterStateError> {
        let root_id = self
            .get()
            .root_pid
            .ok_or(CanisterStateError::RootIdNotSet)?;

        Ok(root_id)
    }

    // set_root_pid
    pub fn set_root_pid(&mut self, pid: Principal) -> Result<(), CanisterStateError> {
        let mut state = self.get();
        state.root_pid = Some(pid);
        self.set(state)?;

        Ok(())
    }

    // get_parent_pid
    #[must_use]
    pub fn get_parent_pid(&self) -> Option<Principal> {
        self.get().parent_pid
    }

    // set_parent_pid
    pub fn set_parent_pid(&mut self, id: Principal) -> Result<(), CanisterStateError> {
        let mut state = self.get();
        state.parent_pid = Some(id);
        self.set(state)?;

        Ok(())
    }
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Default, Serialize, Deserialize)]
pub struct CanisterStateData {
    path: Option<String>,
    root_pid: Option<Principal>,
    parent_pid: Option<Principal>,
}

impl_storable_unbounded!(CanisterStateData);
