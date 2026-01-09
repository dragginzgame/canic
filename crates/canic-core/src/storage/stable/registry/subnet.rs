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
//!
//! Non-invariants (caller responsibility):
//! - Role uniqueness
//! - Root singularity

use crate::{
    cdk::{
        candid::Principal,
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    storage::{canister::CanisterRecord, stable::memory::registry::SUBNET_REGISTRY_ID},
};
use std::cell::RefCell;

eager_static! {
    static SUBNET_REGISTRY: RefCell<
        BTreeMap<Principal, CanisterRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(BTreeMap::init(
        ic_memory!(SubnetRegistry, SUBNET_REGISTRY_ID)
    ));
}

///
/// Snapshot of registry contents (for export / tests)
///
#[derive(Clone, Debug)]
pub struct SubnetRegistryData {
    pub entries: Vec<(Principal, CanisterRecord)>,
}

///
/// SubnetRegistry
///
pub struct SubnetRegistry;

impl SubnetRegistry {
    //
    // Core accessors
    //

    /// Returns the record for the given canister, if present.
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterRecord> {
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

    /// Registers a new non-root canister.
    pub(crate) fn register(
        pid: Principal,
        role: &CanisterRole,
        parent_pid: Principal,
        module_hash: Vec<u8>,
        created_at: u64,
    ) {
        let record = CanisterRecord {
            role: role.clone(),
            parent_pid: Some(parent_pid),
            module_hash: Some(module_hash),
            created_at,
        };

        Self::insert(pid, record);
    }

    /// Registers the root canister.
    pub(crate) fn register_root(pid: Principal, created_at: u64) {
        let record = CanisterRecord {
            role: CanisterRole::ROOT,
            parent_pid: None,
            module_hash: None,
            created_at,
        };

        Self::insert(pid, record);
    }

    fn insert(pid: Principal, record: CanisterRecord) {
        SUBNET_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(pid, record);
        });
    }

    //
    // Mutation
    //

    /// Updates the recorded module hash.
    /// Returns `true` if the canister existed.
    #[must_use]
    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SUBNET_REGISTRY.with_borrow_mut(|reg| match reg.get(&pid) {
            Some(mut record) => {
                record.module_hash = Some(module_hash);
                reg.insert(pid, record);
                true
            }
            None => false,
        })
    }

    /// Removes a canister entry.
    #[must_use]
    pub(crate) fn remove(pid: &Principal) -> Option<CanisterRecord> {
        SUBNET_REGISTRY.with_borrow_mut(|map| map.remove(pid))
    }

    //
    // Hierarchical queries
    //

    /// Returns all **direct** children of `parent`.
    #[must_use]
    pub(crate) fn children(parent: Principal) -> Vec<(Principal, CanisterRecord)> {
        SUBNET_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter_map(|e| {
                    let pid = *e.key();
                    let record = e.value();

                    if record.parent_pid == Some(parent) {
                        Some((pid, record))
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    //
    // Export
    //

    /// Returns a snapshot of all registry entries.
    #[must_use]
    pub(crate) fn export() -> SubnetRegistryData {
        SUBNET_REGISTRY.with_borrow(|map| SubnetRegistryData {
            entries: map.iter().map(|e| (*e.key(), e.value())).collect(),
        })
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

    fn clear_registry() {
        SUBNET_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }

    fn seed_simple_tree() {
        clear_registry();

        SubnetRegistry::register_root(p(1), 1);
        SubnetRegistry::register(p(2), &CanisterRole::new("alpha"), p(1), vec![], 2);
        SubnetRegistry::register(p(3), &CanisterRole::new("beta"), p(1), vec![], 3);
    }

    #[test]
    fn get_and_get_parent_work() {
        seed_simple_tree();

        let record = SubnetRegistry::get(p(2)).expect("alpha exists");
        assert_eq!(record.parent_pid, Some(p(1)));

        let parent = SubnetRegistry::get_parent(p(2));
        assert_eq!(parent, Some(p(1)));

        assert_eq!(SubnetRegistry::get_parent(p(1)), None);
    }

    #[test]
    fn children_returns_only_direct_children() {
        seed_simple_tree();

        let children = SubnetRegistry::children(p(1));
        let pids: Vec<Principal> = children.into_iter().map(|(pid, _)| pid).collect();

        assert_eq!(pids.len(), 2);
        assert!(pids.contains(&p(2)));
        assert!(pids.contains(&p(3)));
    }

    #[test]
    fn children_of_leaf_is_empty() {
        seed_simple_tree();

        let children = SubnetRegistry::children(p(2));
        assert!(children.is_empty());
    }

    #[test]
    fn update_module_hash_mutates_existing_entry() {
        seed_simple_tree();

        let updated = SubnetRegistry::update_module_hash(p(2), vec![1, 2, 3]);
        assert!(updated);

        let record = SubnetRegistry::get(p(2)).unwrap();
        assert_eq!(record.module_hash, Some(vec![1, 2, 3]));
    }

    #[test]
    fn update_module_hash_returns_false_for_missing_entry() {
        clear_registry();

        let updated = SubnetRegistry::update_module_hash(p(9), vec![1, 2, 3]);
        assert!(!updated);
    }

    #[test]
    fn remove_deletes_entry_and_returns_it() {
        seed_simple_tree();

        let removed = SubnetRegistry::remove(&p(2)).expect("entry removed");

        assert_eq!(removed.parent_pid, Some(p(1)));
        assert!(SubnetRegistry::get(p(2)).is_none());
    }

    #[test]
    fn export_returns_all_entries() {
        seed_simple_tree();

        let exported = SubnetRegistry::export();
        let pids: Vec<Principal> = exported.entries.into_iter().map(|(pid, _)| pid).collect();

        assert_eq!(pids.len(), 3);
        assert!(pids.contains(&p(1)));
        assert!(pids.contains(&p(2)));
        assert!(pids.contains(&p(3)));
    }
}
