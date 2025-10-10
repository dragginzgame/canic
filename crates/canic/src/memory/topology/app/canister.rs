use crate::{
    Error,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{
        CanisterEntry, CanisterStatus, id::topology::app::APP_CANISTER_REGISTRY_ID,
        topology::TopologyError,
    },
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;
use std::cell::RefCell;

//
// APP_CANISTER_REGISTRY
//

eager_static! {
    static APP_CANISTER_REGISTRY: RefCell<BTreeMap<Principal, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(AppCanisterRegistry, APP_CANISTER_REGISTRY_ID)));
}

///
/// AppCanisterRegistry
///

pub struct AppCanisterRegistry;

impl AppCanisterRegistry {
    #[must_use]
    pub fn init_root(pid: Principal) -> CanisterEntry {
        let entry = CanisterEntry {
            pid,
            ty: CanisterType::ROOT,
            parent_pid: None,
            status: CanisterStatus::Installed,
            module_hash: None,
            created_at: now_secs(),
        };

        APP_CANISTER_REGISTRY.with_borrow_mut(|map| map.insert(pid, entry.clone()));

        entry
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterEntry> {
        APP_CANISTER_REGISTRY.with_borrow(|map| map.get(&pid))
    }

    pub fn try_get(pid: Principal) -> Result<CanisterEntry, Error> {
        Self::get(pid).ok_or_else(|| TopologyError::PrincipalNotFound(pid).into())
    }

    /// Look up a canister by its type.
    pub fn try_get_type(ty: &CanisterType) -> Result<CanisterEntry, Error> {
        APP_CANISTER_REGISTRY.with_borrow(|map| {
            map.iter()
                .map(|e| e.value())
                .find(|entry| &entry.ty == ty)
                .ok_or_else(|| TopologyError::TypeNotFound(ty.clone()).into())
        })
    }
}
