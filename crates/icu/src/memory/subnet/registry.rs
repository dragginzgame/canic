use crate::{
    Error, ThisError,
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    config::Config,
    icu_register_memory,
    memory::{
        CanisterEntry, CanisterStatus, CanisterView, MemoryError, SUBNET_REGISTRY_MEMORY_ID,
        subnet::SubnetError,
    },
    types::CanisterType,
    utils::time::now_secs,
};
use candid::Principal;
use std::cell::RefCell;

// thread local
thread_local! {
    static SUBNET_REGISTRY: RefCell<BTreeMap<Principal, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(icu_register_memory!(SUBNET_REGISTRY_MEMORY_ID)));
}

///
/// SubnetRegistryError
///

#[derive(Debug, ThisError)]
pub enum SubnetRegistryError {
    #[error("canister already installed: {0}")]
    AlreadyInstalled(Principal),
}

///
/// SubnetRegistry
///

pub struct SubnetRegistry;

impl SubnetRegistry {
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

        SUBNET_REGISTRY.with_borrow_mut(|map| map.insert(pid, entry.clone()));

        entry
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow(|map| map.get(&pid))
    }

    pub fn try_get(pid: Principal) -> Result<CanisterEntry, Error> {
        Self::get(pid).ok_or_else(|| MemoryError::from(SubnetError::PrincipalNotFound(pid)).into())
    }

    /// Look up a canister by its type.
    pub fn try_get_type(ty: &CanisterType) -> Result<CanisterEntry, Error> {
        SUBNET_REGISTRY.with_borrow(|map| {
            map.iter()
                .map(|e| e.value())
                .find(|entry| &entry.ty == ty)
                .ok_or_else(|| MemoryError::from(SubnetError::TypeNotFound(ty.clone())).into())
        })
    }

    pub fn create(pid: Principal, ty: &CanisterType, parent_pid: Principal) {
        let entry = CanisterEntry {
            pid,
            ty: ty.clone(),
            parent_pid: Some(parent_pid),
            status: CanisterStatus::Created,
            module_hash: None,
            created_at: now_secs(),
        };

        SUBNET_REGISTRY.with_borrow_mut(|map| map.insert(pid, entry));
    }

    pub fn install(pid: Principal, module_hash: Vec<u8>) -> Result<(), Error> {
        SUBNET_REGISTRY.with_borrow_mut(|map| {
            let entry = map
                .get(&pid)
                .ok_or_else(|| MemoryError::from(SubnetError::PrincipalNotFound(pid)))?;

            if entry.status == CanisterStatus::Installed {
                return Err(MemoryError::from(SubnetError::from(
                    SubnetRegistryError::AlreadyInstalled(pid),
                ))
                .into());
            }

            let mut updated = entry;
            updated.status = CanisterStatus::Installed;
            updated.module_hash = Some(module_hash);
            map.insert(pid, updated);
            Ok(())
        })
    }

    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow_mut(|map| map.remove(pid))
    }

    #[must_use]
    pub fn export() -> Vec<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow(|map| map.iter().map(|e| e.value()).collect())
    }

    #[must_use]
    pub fn export_views() -> Vec<CanisterView> {
        SUBNET_REGISTRY
            .with_borrow(|map| map.iter().map(|e| CanisterView::from(e.value())).collect())
    }

    #[must_use]
    pub fn subnet_directory() -> Vec<CanisterView> {
        Self::export()
            .into_iter()
            .filter(|e| e.status == CanisterStatus::Installed)
            .filter(|e| {
                e.ty == CanisterType::ROOT
                    || Config::try_get_canister(&e.ty)
                        .map(|cfg| cfg.uses_directory)
                        .unwrap_or(false)
            })
            .map(CanisterView::from)
            .collect()
    }

    /// Return the full parent chain for a given PID,
    /// starting with the root-most parent and ending with the given canister.
    #[must_use]
    pub fn parents(pid: Principal) -> Vec<CanisterView> {
        let mut result = Vec::new();
        let mut current = Some(pid);

        while let Some(p) = current {
            if let Ok(entry) = Self::try_get(p) {
                let view: CanisterView = entry.clone().into();
                result.push(view);
                current = entry.parent_pid;
            } else {
                break; // orphaned, stop here
            }
        }

        result.reverse();
        result
    }

    /// Return the subtree rooted at `pid`:
    /// the original canister (if found) plus all its descendants.
    #[must_use]
    pub fn subtree(pid: Principal) -> Vec<CanisterView> {
        let mut result = vec![];

        if let Ok(entry) = Self::try_get(pid) {
            result.push(entry.into());
        }

        let mut stack = vec![pid];
        while let Some(current) = stack.pop() {
            let children: Vec<CanisterView> = Self::export()
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
    pub fn is_in_subtree(root_pid: Principal, entry: &CanisterView, all: &[CanisterView]) -> bool {
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

    #[cfg(test)]
    pub fn clear_for_tests() {
        SUBNET_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }
}
