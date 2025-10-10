use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_bounded,
    memory::{MemoryError, context::ContextError, id::context::CANISTER_CONTEXT_ID},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_CONTEXT
//

eager_static! {
    static CANISTER_CONTEXT: RefCell<Cell<CanisterContextData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(CanisterContext, CANISTER_CONTEXT_ID),
            CanisterContextData::default(),
        ));
}

///
/// CanisterContextError
///

#[derive(Debug, ThisError)]
pub enum CanisterContextError {
    #[error("root pid has not been set")]
    RootNotSet,
}

impl From<CanisterContextError> for Error {
    fn from(err: CanisterContextError) -> Self {
        MemoryError::from(ContextError::from(err)).into()
    }
}

///
/// CanisterContextData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct CanisterContextData {
    pub root_pid: Option<Principal>,
}

impl_storable_bounded!(CanisterContextData, 32, true);

///
/// CanisterContext
///

pub struct CanisterContext;

impl CanisterContext {
    // ---- Root PID ----

    #[must_use]
    pub fn get_root_pid() -> Option<Principal> {
        CANISTER_CONTEXT.with_borrow(|cell| cell.get().root_pid)
    }

    pub fn try_get_root_pid() -> Result<Principal, Error> {
        let pid = Self::get_root_pid().ok_or(CanisterContextError::RootNotSet)?;

        Ok(pid)
    }

    pub fn set_root_pid(pid: Principal) {
        CANISTER_CONTEXT.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.root_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Import / Export ----

    pub fn import(data: CanisterContextData) {
        CANISTER_CONTEXT.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub fn export() -> CanisterContextData {
        CANISTER_CONTEXT.with_borrow(|cell| *cell.get())
    }
}
