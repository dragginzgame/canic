use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory,
    storage::{prelude::*, stable::memory::env::ENV_ID},
};
use std::cell::RefCell;

eager_static! {
    //
    // ENV
    // All the environment variables a canister needs
    //
    static ENV: RefCell<Cell<EnvData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(EnvData, ENV_ID),
            EnvData::default(),
        ));
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EnvData {
    // app
    pub prime_root_pid: Option<Principal>,

    // subnet
    pub subnet_role: Option<SubnetRole>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

impl_storable_bounded!(EnvData, 256, true);

///
/// Env
///

pub struct Env;

impl Env {
    #[must_use]
    pub(crate) fn get_prime_root_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().prime_root_pid)
    }

    #[must_use]
    pub(crate) fn get_subnet_role() -> Option<SubnetRole> {
        ENV.with_borrow(|cell| cell.get().subnet_role.clone())
    }

    #[must_use]
    pub(crate) fn get_subnet_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().subnet_pid)
    }

    pub(crate) fn set_subnet_pid(pid: Principal) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.subnet_pid = Some(pid);
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn get_root_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().root_pid)
    }

    #[must_use]
    pub(crate) fn get_canister_role() -> Option<CanisterRole> {
        ENV.with_borrow(|cell| cell.get().canister_role.clone())
    }

    /// Set/replace the current canister role.
    pub(crate) fn set_canister_role(role: CanisterRole) {
        ENV.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.canister_role = Some(role);
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn get_parent_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().parent_pid)
    }

    // ---- Import / Export ----

    /// Import a complete EnvData record, replacing any existing state.
    pub(crate) fn import(data: EnvData) {
        ENV.with_borrow_mut(|cell| {
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn export() -> EnvData {
        ENV.with_borrow(|cell| cell.get().clone())
    }
}
