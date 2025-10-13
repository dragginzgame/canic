use crate::{
    Error, ThisError,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::{
        CanisterEntry, CanisterSummary, id::topology::subnet::SUBNET_CANISTER_REGISTRY_ID,
        topology::TopologyError,
    },
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;
use std::cell::RefCell;

//
// SUBNET_CANISTER_REGISTRY
//

eager_static! {
    static SUBNET_CANISTER_REGISTRY: RefCell<BTreeMap<Principal, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetCanisterRegistry, SUBNET_CANISTER_REGISTRY_ID)));
}

///
/// SubnetCanisterRegistryError
///

#[derive(Debug, ThisError)]
pub enum SubnetCanisterRegistryError {}

///
/// SubnetCanisterRegistry
///

pub struct SubnetCanisterRegistry;

impl SubnetCanisterRegistry {
    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterEntry> {
        SUBNET_CANISTER_REGISTRY.with_borrow(|map| map.get(&pid))
    }

    pub fn try_get(pid: Principal) -> Result<CanisterEntry, Error> {
        Self::get(pid).ok_or_else(|| TopologyError::PrincipalNotFound(pid).into())
    }

    /// Look up a canister by its type.
    pub fn try_get_type(ty: &CanisterType) -> Result<CanisterEntry, Error> {
        SUBNET_CANISTER_REGISTRY.with_borrow(|map| {
            map.iter()
                .map(|e| e.value())
                .find(|entry| &entry.ty == ty)
                .ok_or_else(|| TopologyError::TypeNotFound(ty.clone()).into())
        })
    }

    /// Register a new canister (non-root) with parent + module hash.
    pub fn register(
        pid: Principal,
        ty: &CanisterType,
        parent_pid: Principal,
        module_hash: Vec<u8>,
    ) {
        let entry = CanisterEntry {
            pid,
            ty: ty.clone(),
            parent_pid: Some(parent_pid),
            module_hash: Some(module_hash),
            created_at: now_secs(),
        };

        Self::insert(entry);
    }

    /// Register the root canister itself (no parent, no module hash).
    pub fn register_root(pid: Principal) {
        let entry = CanisterEntry {
            pid,
            ty: CanisterType::ROOT,
            parent_pid: None,
            module_hash: None,
            created_at: now_secs(),
        };

        Self::insert(entry);
    }

    /// Internal helper: inserts a fully-formed entry into the registry.
    fn insert(entry: CanisterEntry) {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(entry.pid, entry);
        });
    }

    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(|map| map.remove(pid))
    }

    #[must_use]
    pub fn all() -> Vec<CanisterEntry> {
        SUBNET_CANISTER_REGISTRY.with_borrow(|map| map.iter().map(|e| e.value()).collect())
    }

    #[cfg(test)]
    pub fn clear_for_tests() {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }

    /// Return the direct children of the given `pid`.
    ///
    /// This only returns canisters whose `parent_pid` is exactly `pid`
    /// (one level down). It does not recurse into grandchildren.
    #[must_use]
    pub fn children(pid: Principal) -> Vec<CanisterSummary> {
        Self::all()
            .into_iter()
            .filter(|e| e.parent_pid == Some(pid))
            .map(Into::into)
            .collect()
    }

    /// Return the subtree rooted at `pid`:
    /// the original canister (if found) plus all its descendants.
    #[must_use]
    pub fn subtree(pid: Principal) -> Vec<CanisterSummary> {
        let mut result = vec![];

        if let Ok(entry) = Self::try_get(pid) {
            result.push(entry.into());
        }

        let mut stack = vec![pid];
        while let Some(current) = stack.pop() {
            let children: Vec<CanisterSummary> = Self::all()
                .into_iter()
                .filter(|e| e.parent_pid == Some(current))
                .map(Into::into)
                .collect();

            stack.extend(children.iter().map(|c| c.pid));
            result.extend(children);
        }

        result
    }

    /// Return true if `entry` is part of the subtree rooted at `root_pid`.
    #[must_use]
    pub fn is_in_subtree(
        root_pid: Principal,
        entry: &CanisterSummary,
        all: &[CanisterSummary],
    ) -> bool {
        if entry.pid == root_pid {
            return true;
        }

        let mut current = entry.parent_pid;
        while let Some(pid) = current {
            if pid == root_pid {
                return true;
            }
            current = all.iter().find(|e| e.pid == pid).and_then(|e| e.parent_pid);
        }

        false
    }
}
