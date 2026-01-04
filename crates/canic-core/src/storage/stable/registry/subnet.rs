//! SubnetRegistry
//!
//! Authoritative persistent registry of subnet canisters and their
//! hierarchical relationships.
//!
//! Invariants:
//! - Each canister has at most one parent.
//! - The root canister has no parent.
//! - Parent relationships may form arbitrary DAGs; cycles are tolerated
//!   and handled defensively during traversal.
//! - This module does not enforce role uniqueness or root singularity;
//!   callers are responsible for maintaining those invariants.

use crate::{
    cdk::{
        candid::Principal,
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    storage::{
        canister::{CanisterEntry, CanisterSummary},
        stable::memory::registry::SUBNET_REGISTRY_ID,
    },
};
use std::cell::RefCell;

eager_static! {
    static SUBNET_REGISTRY: RefCell<BTreeMap<Principal, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetRegistry, SUBNET_REGISTRY_ID)));
}

///
/// SubnetRegistryData
///

#[derive(Clone, Debug)]
pub struct SubnetRegistryData {
    pub entries: Vec<(Principal, CanisterEntry)>,
}

///
/// SubnetRegistry
///

pub struct SubnetRegistry;

impl SubnetRegistry {
    //
    // Internal helper
    //

    fn with_entries<F, R>(f: F) -> R
    where
        F: FnOnce(
            &mut ic_stable_structures::btreemap::Iter<
                Principal,
                CanisterEntry,
                VirtualMemory<DefaultMemoryImpl>,
            >,
        ) -> R,
    {
        SUBNET_REGISTRY.with_borrow(|map| {
            let mut iter = map.iter();
            f(&mut iter)
        })
    }

    //
    // Core accessors
    //

    /// Returns a canister entry for the given [`Principal`], if present.
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow(|map| map.get(&pid))
    }

    /// Returns the parent PID for a given canister, if recorded.
    #[must_use]
    pub(crate) fn get_parent(pid: Principal) -> Option<Principal> {
        Self::get(pid)?.parent_pid
    }

    //
    // Registration
    //

    /// Registers a new non-root canister with its parent and module hash.
    pub(crate) fn register(
        pid: Principal,
        role: &CanisterRole,
        parent_pid: Principal,
        module_hash: Vec<u8>,
        created_at: u64,
    ) {
        let entry = CanisterEntry {
            role: role.clone(),
            parent_pid: Some(parent_pid),
            module_hash: Some(module_hash),
            created_at,
        };

        Self::insert(pid, entry);
    }

    /// Register the root canister itself (no parent, no module hash).
    pub(crate) fn register_root(pid: Principal, created_at: u64) {
        let entry = CanisterEntry {
            role: CanisterRole::ROOT,
            parent_pid: None,
            module_hash: None,
            created_at,
        };

        Self::insert(pid, entry);
    }

    /// Inserts a fully formed entry into the registry.
    fn insert(pid: Principal, entry: CanisterEntry) {
        SUBNET_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(pid, entry);
        });
    }

    /// Update the recorded module hash for a canister, returning whether it existed.
    #[must_use]
    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SUBNET_REGISTRY.with_borrow_mut(|reg| {
            if let Some(mut entry) = reg.get(&pid) {
                entry.module_hash = Some(module_hash);
                reg.insert(pid, entry);
                true
            } else {
                false
            }
        })
    }

    /// Removes a canister entry by principal.
    #[must_use]
    pub(crate) fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SUBNET_REGISTRY.with_borrow_mut(|map| map.remove(pid))
    }

    //
    // Hierarchical queries
    //

    /// Returns all direct children of a given parent canister (`pid`).
    ///
    /// This only traverses **one level down**.
    #[must_use]
    pub(crate) fn children(parent: Principal) -> Vec<(Principal, CanisterSummary)> {
        Self::with_entries(|iter| {
            iter.filter_map(|e| {
                let pid = *e.key();
                let entry = e.value();
                (entry.parent_pid == Some(parent)).then(|| (pid, CanisterSummary::from(entry)))
            })
            .collect()
        })
    }

    //
    // Export & test utils
    //

    /// Returns all canister entries as a vector.
    #[must_use]
    pub(crate) fn export() -> SubnetRegistryData {
        SubnetRegistryData {
            entries: Self::with_entries(|iter| iter.map(|e| (*e.key(), e.value())).collect()),
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CanisterRole;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn clear() {
        SUBNET_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }

    fn seed_basic_tree() {
        clear();

        SubnetRegistry::register_root(p(1), 1);
        SubnetRegistry::register(p(2), &CanisterRole::new("alpha"), p(1), vec![], 2);
        SubnetRegistry::register(p(3), &CanisterRole::new("beta"), p(1), vec![], 3);
    }

    #[test]
    fn get_and_get_parent_work() {
        seed_basic_tree();

        let entry = SubnetRegistry::get(p(2)).expect("alpha exists");
        assert_eq!(entry.parent_pid, Some(p(1)));

        let parent = SubnetRegistry::get_parent(p(2));
        assert_eq!(parent, Some(p(1)));

        assert_eq!(SubnetRegistry::get_parent(p(1)), None);
    }

    #[test]
    fn children_returns_only_direct_children() {
        seed_basic_tree();

        let children = SubnetRegistry::children(p(1));
        let pids: Vec<Principal> = children.into_iter().map(|(pid, _)| pid).collect();

        assert_eq!(pids.len(), 2);
        assert!(pids.contains(&p(2)));
        assert!(pids.contains(&p(3)));
    }

    #[test]
    fn update_module_hash_mutates_existing_entry() {
        seed_basic_tree();

        let updated = SubnetRegistry::update_module_hash(p(2), vec![1, 2, 3]);
        assert!(updated);

        let entry = SubnetRegistry::get(p(2)).unwrap();
        assert_eq!(entry.module_hash, Some(vec![1, 2, 3]));
    }

    #[test]
    fn update_module_hash_returns_false_for_missing_entry() {
        clear();

        let updated = SubnetRegistry::update_module_hash(p(9), vec![1, 2, 3]);
        assert!(!updated);
    }

    #[test]
    fn remove_deletes_entry_and_returns_it() {
        seed_basic_tree();

        let removed = SubnetRegistry::remove(&p(2)).expect("entry removed");

        assert_eq!(removed.parent_pid, Some(p(1)));
        assert!(SubnetRegistry::get(p(2)).is_none());
    }

    #[test]
    fn export_returns_all_entries() {
        seed_basic_tree();

        let exported = SubnetRegistry::export();
        let pids: Vec<Principal> = exported.entries.into_iter().map(|(pid, _)| pid).collect();

        assert_eq!(pids.len(), 3);
        assert!(pids.contains(&p(1)));
        assert!(pids.contains(&p(2)));
        assert!(pids.contains(&p(3)));
    }
}
