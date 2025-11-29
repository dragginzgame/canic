use crate::{
    Error,
    cdk::{
        api::canister_self,
        structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    },
    eager_static, ic_memory, impl_storable_bounded,
    model::{
        ModelError,
        memory::{MemoryError, id::ENV_ID},
    },
    types::{CanisterType, SubnetType},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// ENV
// All the environment variables a canister needs
//

eager_static! {
    static ENV: RefCell<Cell<EnvData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(EnvData, ENV_ID),
            EnvData::default(),
        ));
}

///
/// ContextError
///

#[derive(Debug, ThisError)]
pub enum ContextError {
    #[error("canister type not set")]
    CanisterTypeNotSet,

    #[error("prime root pid not set")]
    PrimeRootPidNotSet,

    #[error("root pid not set")]
    RootPidNotSet,

    #[error("subnet pid not set")]
    SubnetPidNotSet,

    #[error("subnet type not set")]
    SubnetTypeNotSet,
}

impl From<ContextError> for Error {
    fn from(err: ContextError) -> Self {
        ModelError::MemoryError(MemoryError::from(err)).into()
    }
}

///
/// EnvData
///
/// `prime_root_pid` : passed to the root during install arguments.
/// `parent_pid`     : passed to the root during install arguments.
///
/// All other fields are derived during install/upgrade and cached locally so
/// every canister can answer questions about its environment without touching
/// global state.
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct EnvData {
    // app
    pub prime_root_pid: Option<Principal>,

    // subnet
    pub subnet_type: Option<SubnetType>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_type: Option<CanisterType>,
    pub parent_pid: Option<Principal>,
}

impl_storable_bounded!(EnvData, 256, true);

///
/// Env
///

pub struct Env;

impl Env {
    // ---- Prime Root PID ----
    #[must_use]
    pub fn get_prime_root_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().prime_root_pid)
    }

    pub fn try_get_prime_root_pid() -> Result<Principal, Error> {
        Self::get_prime_root_pid().ok_or_else(|| ContextError::PrimeRootPidNotSet.into())
    }

    pub fn set_prime_root_pid(pid: Principal) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.prime_root_pid = Some(pid);
            cell.set(data);
        });
    }

    #[must_use]
    pub fn is_prime_root() -> bool {
        let prime_root_pid = Self::get_prime_root_pid();

        prime_root_pid.is_some() && prime_root_pid == Self::get_root_pid()
    }

    // ---- Subnet Type ----
    #[must_use]
    pub fn get_subnet_type() -> Option<SubnetType> {
        ENV.with_borrow(|cell| cell.get().subnet_type.clone())
    }

    pub fn try_get_subnet_type() -> Result<SubnetType, Error> {
        Self::get_subnet_type().ok_or_else(|| ContextError::SubnetTypeNotSet.into())
    }

    pub fn set_subnet_type(ty: SubnetType) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.subnet_type = Some(ty);
            cell.set(data);
        });
    }

    // ---- Subnet PID ----
    #[must_use]
    pub fn get_subnet_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().subnet_pid)
    }

    pub fn try_get_subnet_pid() -> Result<Principal, Error> {
        Self::get_subnet_pid().ok_or_else(|| ContextError::SubnetPidNotSet.into())
    }

    pub fn set_subnet_pid(pid: Principal) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.subnet_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Root PID ----

    #[must_use]
    pub fn get_root_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().root_pid)
    }

    pub fn try_get_root_pid() -> Result<Principal, Error> {
        let pid = Self::get_root_pid().ok_or(ContextError::RootPidNotSet)?;

        Ok(pid)
    }

    pub fn set_root_pid(pid: Principal) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.root_pid = Some(pid);
            cell.set(data);
        });
    }

    #[must_use]
    pub fn is_root() -> bool {
        Self::get_root_pid() == Some(canister_self())
    }

    // ---- Canister Type ----

    #[must_use]
    pub fn get_canister_type() -> Option<CanisterType> {
        ENV.with_borrow(|cell| cell.get().canister_type.clone())
    }

    /// Try to get the current canister type, or error if missing.
    pub fn try_get_canister_type() -> Result<CanisterType, Error> {
        Self::get_canister_type().ok_or_else(|| ContextError::CanisterTypeNotSet.into())
    }

    /// Set/replace the current canister type.
    pub fn set_canister_type(ty: CanisterType) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.canister_type = Some(ty);
            cell.set(data);
        });
    }

    // ---- Parent PID ----
    #[must_use]
    pub fn get_parent_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().parent_pid)
    }

    pub fn set_parent_pid(pid: Principal) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.parent_pid = Some(pid);
            cell.set(data);
        });
    }

    // ---- Import / Export ----

    /// Import a complete EnvData record, replacing any existing state.
    pub fn import(data: EnvData) {
        ENV.with_borrow_mut(|cell| {
            cell.set(data);
        });
    }

    #[must_use]
    pub fn export() -> EnvData {
        ENV.with_borrow(|cell| cell.get().clone())
    }
}
