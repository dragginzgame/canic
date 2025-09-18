use crate::{
    Error,
    cdk::structures::{Cell, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_STATE_MEMORY_ID, MemoryError},
    types::CanisterType,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_STATE
//

thread_local! {
    static CANISTER_STATE: RefCell<CanisterStateCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterStateCore::new(Cell::init(
            icu_register_memory!(CANISTER_STATE_MEMORY_ID),
            CanisterStateData::default(),
        )));
}

///
/// CanisterStateError
///

#[derive(Debug, ThisError)]
pub enum CanisterStateError {
    #[error("canister type has not been set")]
    TypeNotSet,
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterStateData {
    pub canister_type: Option<CanisterType>,
}

impl_storable_unbounded!(CanisterStateData);

///
/// CanisterState
///

pub struct CanisterState;

impl CanisterState {
    #[must_use]
    pub fn get_type() -> Option<CanisterType> {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_type)
    }

    pub fn try_get_type() -> Result<CanisterType, Error> {
        Self::get_type().ok_or_else(|| MemoryError::from(CanisterStateError::TypeNotSet).into())
    }

    pub fn set_type(ty: &CanisterType) -> Result<(), Error> {
        CANISTER_STATE.with_borrow_mut(|core| core.set_type(ty))
    }

    #[must_use]
    pub fn is_root() -> bool {
        CANISTER_STATE.with_borrow(|core| core.get_type() == Some(CanisterType::ROOT))
    }

    /// Export full state snapshot
    #[must_use]
    pub fn export() -> CanisterStateData {
        CANISTER_STATE.with_borrow(CanisterStateCore::export)
    }

    /// Import full state snapshot (replace existing)
    pub fn import(data: CanisterStateData) {
        CANISTER_STATE.with_borrow_mut(|core| core.import(data));
    }
}

///
/// CanisterStateCore
///

pub struct CanisterStateCore<M: Memory> {
    cell: Cell<CanisterStateData, M>,
}

impl<M: Memory> CanisterStateCore<M> {
    pub const fn new(cell: Cell<CanisterStateData, M>) -> Self {
        Self { cell }
    }

    pub fn get_type(&self) -> Option<CanisterType> {
        self.cell.get().canister_type.clone()
    }

    // set_type
    // pass by reference required as it's a const
    pub fn set_type(&mut self, ty: &CanisterType) -> Result<(), Error> {
        let mut state = self.cell.get().clone();
        state.canister_type = Some(ty.clone());
        self.cell.set(state);

        Ok(())
    }

    /// Export current state data
    pub fn export(&self) -> CanisterStateData {
        self.cell.get().clone()
    }

    /// Import state data (replace existing)
    pub fn import(&mut self, data: CanisterStateData) {
        self.cell.set(data);
    }
}
