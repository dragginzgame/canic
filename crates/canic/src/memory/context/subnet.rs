use crate::{
    Error,
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory, impl_storable_bounded,
    memory::{MemoryError, context::ContextError, id::context::SUBNET_CONTEXT_ID},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SUBNET_CONTEXT
//

eager_static! {
    static SUBNET_CONTEXT: RefCell<Cell<SubnetContextData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(SubnetState, SUBNET_CONTEXT_ID),
            SubnetContextData::default(),
        ));
}

///
/// SubnetContextError
///

#[derive(Debug, ThisError)]
pub enum SubnetContextError {
    #[error("prime subnet pid has not been set")]
    PrimeSubnetNotSet,

    #[error("subnet pid has not been set")]
    SubnetNotSet,
}

impl From<SubnetContextError> for Error {
    fn from(err: SubnetContextError) -> Self {
        MemoryError::from(ContextError::from(err)).into()
    }
}

///
/// SubnetContextData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetContextData {
    pub prime_subnet_pid: Option<Principal>,
    pub subnet_pid: Option<Principal>,
}

impl_storable_bounded!(SubnetContextData, 64, true);

///
/// SubnetContext
///

pub struct SubnetContext;

impl SubnetContext {
    // ---- Subnet PID ----
    #[must_use]
    pub fn get_subnet_pid() -> Option<Principal> {
        SUBNET_CONTEXT.with_borrow(|cell| cell.get().subnet_pid)
    }

    pub fn try_get_subnet_pid() -> Result<Principal, Error> {
        Self::get_subnet_pid().ok_or_else(|| SubnetContextError::SubnetNotSet.into())
    }

    pub fn set_subnet_pid(pid: Principal) {
        SUBNET_CONTEXT.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.subnet_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Prime Subnet PID ----
    #[must_use]
    pub fn get_prime_subnet_pid() -> Option<Principal> {
        SUBNET_CONTEXT.with_borrow(|cell| cell.get().prime_subnet_pid)
    }

    pub fn try_get_prime_subnet_pid() -> Result<Principal, Error> {
        Self::get_prime_subnet_pid().ok_or_else(|| SubnetContextError::PrimeSubnetNotSet.into())
    }

    pub fn set_prime_subnet_pid(pid: Principal) {
        SUBNET_CONTEXT.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.prime_subnet_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Import / Export ----

    #[must_use]
    pub fn export() -> SubnetContextData {
        SUBNET_CONTEXT.with_borrow(|cell| *cell.get())
    }
}
