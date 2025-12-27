use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        utils::time::now_secs,
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    model::memory::{CanisterEntry, CanisterSummary, id::registry::SUBNET_REGISTRY_ID},
};
use candid::Principal;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

//
// SUBNET_REGISTRY
//

eager_static! {
    static SUBNET_REGISTRY: RefCell<BTreeMap<Principal, CanisterEntry, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(SubnetCanisterRegistry, SUBNET_REGISTRY_ID)));
}

///
/// SubnetCanisterRegistryView
///

pub type SubnetCanisterRegistryView = Vec<CanisterEntry>;

///
/// SubnetCanisterRegistry
///

pub struct SubnetCanisterRegistry;

impl SubnetCanisterRegistry {
    //
    // Internal helper
    //

    fn with_entries<F, R>(f: F) -> R
    where
        F: FnOnce(
            ic_stable_structures::btreemap::Iter<
                Principal,
                CanisterEntry,
                VirtualMemory<DefaultMemoryImpl>,
            >,
        ) -> R,
    {
        SUBNET_CANISTER_REGISTRY.with_borrow(|map| f(map.iter()))
    }

    //
    // Core accessors
    //

    /// Returns a canister entry for the given [`Principal`], if present.
    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterEntry> {
        SUBNET_CANISTER_REGISTRY.with_borrow(|map| map.get(&pid))
    }

    /// Returns the parent PID for a given canister, if recorded.
    #[must_use]
    pub(crate) fn get_parent(pid: Principal) -> Option<Principal> {
        Self::get(pid)?.parent_pid
    }

    /// Finds the first canister with the given [`CanisterRole`].
    #[must_use]
    pub(crate) fn find_first_by_role(role: &CanisterRole) -> Option<CanisterEntry> {
        Self::with_entries(|iter| iter.map(|e| e.value()).find(|entry| &entry.role == role))
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
    ) {
        let entry = CanisterEntry {
            pid,
            role: role.clone(),
            parent_pid: Some(parent_pid),
            module_hash: Some(module_hash),
            created_at: now_secs(),
        };

        Self::insert(entry);
    }

    /// Register the root canister itself (no parent, no module hash).
    pub(crate) fn register_root(pid: Principal) {
        let entry = CanisterEntry {
            pid,
            role: CanisterRole::ROOT,
            parent_pid: None,
            module_hash: None,
            created_at: now_secs(),
        };

        Self::insert(entry);
    }

    /// Inserts a fully formed entry into the registry.
    fn insert(entry: CanisterEntry) {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(entry.pid, entry);
        });
    }

    /// Update the recorded module hash for a canister, returning whether it existed.
    #[must_use]
    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(|reg| {
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
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(|map| map.remove(pid))
    }

    //
    // Hierarchical queries
    //

    /// Returns all direct children of a given parent canister (`pid`).
    ///
    /// This only traverses **one level down**.
    #[must_use]
    pub(crate) fn children(pid: Principal) -> Vec<CanisterSummary> {
        Self::with_entries(|iter| {
            iter.filter_map(|entry| {
                let value = entry.value();
                (value.parent_pid == Some(pid)).then(|| CanisterSummary::from(value))
            })
            .collect()
        })
    }

    /// Returns the entire subtree rooted at `pid`:
    /// the original canister (if found) plus all its descendants.
    #[must_use]
    pub(crate) fn subtree(pid: Principal) -> Vec<CanisterSummary> {
        let entries: Vec<CanisterEntry> = Self::export();

        let mut children: HashMap<Principal, Vec<CanisterEntry>> = HashMap::new();
        let mut root: Option<CanisterEntry> = None;

        for e in entries {
            if e.pid == pid {
                root = Some(e.clone());
            }

            if let Some(parent) = e.parent_pid {
                children.entry(parent).or_default().push(e);
            }
        }

        let Some(root) = root else { return vec![] };

        let mut result = vec![root];
        let mut stack = vec![pid];
        let mut visited: HashSet<Principal> = HashSet::new();
        visited.insert(pid);

        while let Some(current) = stack.pop() {
            if let Some(kids) = children.get(&current) {
                for child in kids {
                    if visited.insert(child.pid) {
                        stack.push(child.pid);
                        result.push(child.clone());
                    }
                }
            }
        }

        // Final output → summaries
        result.into_iter().map(CanisterSummary::from).collect()
    }

    /// Return true if `entry` is part of the subtree rooted at `root_pid`.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_in_subtree(
        root_pid: Principal,
        entry: &CanisterSummary,
        all: &[CanisterSummary],
    ) -> bool {
        if entry.pid == root_pid {
            return true;
        }

        let parent_map: HashMap<Principal, Principal> = all
            .iter()
            .filter_map(|e| e.parent_pid.map(|parent| (e.pid, parent)))
            .collect();

        let mut current = entry.parent_pid;
        while let Some(pid) = current {
            if pid == root_pid {
                return true;
            }
            current = parent_map.get(&pid).copied();
        }

        false
    }

    //
    // Export & test utils
    //

    /// Returns all canister entries as a vector.
    #[must_use]
    pub(crate) fn export() -> Vec<CanisterEntry> {
        Self::with_entries(|iter| iter.map(|e| e.value()).collect())
    }

    #[cfg(test)]
    pub(crate) fn clear_for_tests() {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }

    #[cfg(test)]
    pub(crate) fn insert_for_tests(entry: CanisterEntry) {
        SUBNET_CANISTER_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(entry.pid, entry);
        });
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

    fn sort_by_pid(entries: &mut [CanisterSummary]) {
        entries.sort_by(|a, b| a.pid.as_slice().cmp(b.pid.as_slice()));
    }

    fn register_tree() {
        SubnetCanisterRegistry::clear_for_tests();

        // root
        SubnetCanisterRegistry::register_root(p(1));

        // children of root
        SubnetCanisterRegistry::register(p(2), &CanisterRole::new("alpha"), p(1), vec![]);
        SubnetCanisterRegistry::register(p(3), &CanisterRole::new("beta"), p(1), vec![]);

        // grandchildren under alpha
        SubnetCanisterRegistry::register(p(4), &CanisterRole::new("alpha-a"), p(2), vec![]);
        SubnetCanisterRegistry::register(p(5), &CanisterRole::new("alpha-b"), p(2), vec![]);

        // great-grandchild under alpha-b
        SubnetCanisterRegistry::register(p(6), &CanisterRole::new("alpha-b-i"), p(5), vec![]);

        // child under beta
        SubnetCanisterRegistry::register(p(7), &CanisterRole::new("beta-a"), p(3), vec![]);
    }

    #[test]
    fn subtree_handles_unbalanced_tree() {
        register_tree();

        let mut subtree = SubnetCanisterRegistry::subtree(p(2));
        sort_by_pid(&mut subtree);

        let expected: Vec<Principal> = vec![p(2), p(4), p(5), p(6)];
        let actual: Vec<Principal> = subtree.into_iter().map(|e| e.pid).collect();

        assert_eq!(expected, actual);
    }

    #[test]
    fn is_in_subtree_uses_parent_chain_map() {
        register_tree();

        let full_tree = SubnetCanisterRegistry::subtree(p(1));
        let leaf = full_tree.iter().find(|e| e.pid == p(6)).unwrap();
        let sibling = full_tree.iter().find(|e| e.pid == p(7)).unwrap();

        assert!(SubnetCanisterRegistry::is_in_subtree(
            p(2),
            leaf,
            &full_tree
        ));
        assert!(!SubnetCanisterRegistry::is_in_subtree(
            p(2),
            sibling,
            &full_tree
        ));
        assert!(SubnetCanisterRegistry::is_in_subtree(
            p(1),
            sibling,
            &full_tree
        ));
    }

    #[test]
    fn subtree_skips_cycles() {
        SubnetCanisterRegistry::clear_for_tests();

        let a = p(1);
        let b = p(2);

        SubnetCanisterRegistry::insert_for_tests(CanisterEntry {
            pid: a,
            role: CanisterRole::new("alpha"),
            parent_pid: Some(b),
            module_hash: None,
            created_at: now_secs(),
        });
        SubnetCanisterRegistry::insert_for_tests(CanisterEntry {
            pid: b,
            role: CanisterRole::new("beta"),
            parent_pid: Some(a),
            module_hash: None,
            created_at: now_secs(),
        });

        let mut subtree = SubnetCanisterRegistry::subtree(a);
        sort_by_pid(&mut subtree);

        let expected: Vec<Principal> = vec![a, b];
        let actual: Vec<Principal> = subtree.into_iter().map(|e| e.pid).collect();

        assert_eq!(expected, actual);
    }
}
