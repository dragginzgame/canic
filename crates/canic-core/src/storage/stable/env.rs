use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    role_contract::allocation::memory::env::ENV_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

eager_static! {
    //
    // ENV
    // All the environment variables a canister needs
    //
    static ENV: RefCell<Cell<EnvRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.env.v1", ty = EnvRecord, id = ENV_ID),
            EnvRecord::default(),
        ));
}

///
/// EnvRecord
///
/// `prime_root_pid` : passed to the root during install arguments.
/// `parent_pid`     : passed to the root during install arguments.
///
/// All other fields are derived during install/upgrade and cached locally so
/// every canister can answer questions about its environment without touching
/// global state.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EnvRecord {
    // app
    pub prime_root_pid: Option<Principal>,

    // subnet
    pub subnet_role: Option<SubnetSlotId>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

impl_storable_bounded!(EnvRecord, 256, true);

impl EnvRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "EnvRecord";
}

///
/// EnvData
///
/// Canonical environment import/export snapshot.
///

#[derive(Clone, Debug, Default)]
pub struct EnvData {
    pub record: EnvRecord,
}

impl EnvData {
    pub const STATE_CONTRACT_NAME: &'static str = "EnvData";
}

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
    pub(crate) fn get_subnet_role() -> Option<SubnetSlotId> {
        ENV.with_borrow(|cell| cell.get().subnet_role.clone())
    }

    #[must_use]
    pub(crate) fn get_subnet_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().subnet_pid)
    }

    pub(crate) fn set_subnet_pid(pid: Principal) {
        ENV.with_borrow_mut(|cell| {
            let data = cell.get();
            if data.subnet_pid.as_ref() == Some(&pid) {
                return;
            }
            let mut updated = data.clone();
            updated.subnet_pid = Some(pid);
            cell.set(updated);
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
            let data = cell.get();
            if data.canister_role.as_ref() == Some(&role) {
                return;
            }
            let mut updated = data.clone();
            updated.canister_role = Some(role);
            cell.set(updated);
        });
    }

    #[must_use]
    pub(crate) fn get_parent_pid() -> Option<Principal> {
        ENV.with_borrow(|cell| cell.get().parent_pid)
    }

    // ---- Import / Export ----

    /// Import a complete environment snapshot, replacing any existing state.
    pub(crate) fn import(data: EnvData) {
        ENV.with_borrow_mut(|cell| {
            cell.set(data.record);
        });
    }

    #[must_use]
    pub(crate) fn export() -> EnvData {
        EnvData {
            record: ENV.with_borrow(|cell| cell.get().clone()),
        }
    }
}
