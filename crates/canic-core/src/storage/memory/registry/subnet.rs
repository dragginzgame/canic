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
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::CanisterRole,
    storage::{
        canister::{CanisterEntry, CanisterSummary},
        memory::id::registry::SUBNET_REGISTRY_ID,
    },
};
use candid::Principal;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

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

    /// Finds the first canister with the given [`CanisterRole`].
    #[must_use]
    pub(crate) fn find_first_by_role(role: &CanisterRole) -> Option<(Principal, CanisterEntry)> {
        Self::with_entries(|iter| {
            iter.find_map(|e| {
                let v = e.value();
                (&v.role == role).then(|| (*e.key(), v))
            })
        })
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

    /// Returns the entire subtree rooted at `pid`:
    /// the original canister (if found) plus all its descendants.
    #[must_use]
    pub(crate) fn subtree(root_pid: Principal) -> Vec<(Principal, CanisterSummary)> {
        let entries = Self::export();

        // Build parent -> children map
        let mut children: HashMap<Principal, Vec<(Principal, CanisterEntry)>> = HashMap::new();
        let mut root: Option<CanisterEntry> = None;

        for (pid, entry) in entries.entries {
            if pid == root_pid {
                root = Some(entry.clone());
            }

            if let Some(parent) = entry.parent_pid {
                children.entry(parent).or_default().push((pid, entry));
            }
        }

        let Some(root_entry) = root else {
            return vec![];
        };

        let mut result: Vec<(Principal, CanisterSummary)> =
            vec![(root_pid, CanisterSummary::from(root_entry))];

        let mut stack = vec![root_pid];
        let mut visited: HashSet<Principal> = HashSet::new();
        visited.insert(root_pid);

        while let Some(current) = stack.pop() {
            if let Some(kids) = children.get(&current) {
                for (child_pid, child_entry) in kids {
                    if visited.insert(*child_pid) {
                        stack.push(*child_pid);
                        result.push((*child_pid, CanisterSummary::from(child_entry.clone())));
                    }
                }
            }
        }

        result
    }

    /// Return true if `entry_pid` is part of the subtree rooted at `root_pid`.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_in_subtree(
        root_pid: Principal,
        entry_pid: Principal,
        all: &[(Principal, CanisterSummary)],
    ) -> bool {
        if entry_pid == root_pid {
            return true;
        }

        // Build child -> parent map
        let parent_map: HashMap<Principal, Principal> = all
            .iter()
            .filter_map(|(pid, summary)| summary.parent_pid.map(|parent| (*pid, parent)))
            .collect();

        let mut current = parent_map.get(&entry_pid).copied();
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
    use crate::{cdk::utils::time::now_secs, ids::CanisterRole};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sort_by_pid(entries: &mut [(Principal, CanisterSummary)]) {
        entries.sort_by(|(a, _), (b, _)| a.as_slice().cmp(b.as_slice()));
    }

    fn register_tree() {
        // clear registry
        SUBNET_REGISTRY.with_borrow_mut(BTreeMap::clear);

        // root
        SubnetRegistry::register_root(p(1), 1);

        // children of root
        SubnetRegistry::register(p(2), &CanisterRole::new("alpha"), p(1), vec![], 2);
        SubnetRegistry::register(p(3), &CanisterRole::new("beta"), p(1), vec![], 3);

        // grandchildren under alpha
        SubnetRegistry::register(p(4), &CanisterRole::new("alpha-a"), p(2), vec![], 4);
        SubnetRegistry::register(p(5), &CanisterRole::new("alpha-b"), p(2), vec![], 5);

        // great-grandchild under alpha-b
        SubnetRegistry::register(p(6), &CanisterRole::new("alpha-b-i"), p(5), vec![], 6);

        // child under beta
        SubnetRegistry::register(p(7), &CanisterRole::new("beta-a"), p(3), vec![], 7);
    }

    #[test]
    fn subtree_handles_unbalanced_tree() {
        register_tree();

        let mut subtree = SubnetRegistry::subtree(p(2));
        sort_by_pid(&mut subtree);

        let expected = vec![p(2), p(4), p(5), p(6)];
        let actual: Vec<Principal> = subtree.into_iter().map(|(pid, _)| pid).collect();

        assert_eq!(expected, actual);
    }

    #[test]
    fn is_in_subtree_uses_parent_chain_map() {
        register_tree();

        let full_tree = SubnetRegistry::subtree(p(1));

        assert!(SubnetRegistry::is_in_subtree(p(2), p(6), &full_tree));
        assert!(!SubnetRegistry::is_in_subtree(p(2), p(7), &full_tree));
        assert!(SubnetRegistry::is_in_subtree(p(1), p(7), &full_tree));
    }

    #[test]
    fn subtree_skips_cycles() {
        // clear registry
        SUBNET_REGISTRY.with_borrow_mut(BTreeMap::clear);

        let a = p(1);
        let b = p(2);

        SubnetRegistry::insert(
            a,
            CanisterEntry {
                role: CanisterRole::new("alpha"),
                parent_pid: Some(b),
                module_hash: None,
                created_at: now_secs(),
            },
        );

        SubnetRegistry::insert(
            b,
            CanisterEntry {
                role: CanisterRole::new("beta"),
                parent_pid: Some(a),
                module_hash: None,
                created_at: now_secs(),
            },
        );

        let mut subtree = SubnetRegistry::subtree(a);
        sort_by_pid(&mut subtree);

        let expected = vec![a, b];
        let actual: Vec<Principal> = subtree.into_iter().map(|(pid, _)| pid).collect();

        assert_eq!(expected, actual);
    }
}
