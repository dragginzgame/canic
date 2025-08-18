use crate::{
    Error,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory,
    memory::{CHILD_INDEX_MEMORY_ID, MemoryError},
};
use candid::Principal;
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CHILD_INDEX
//

thread_local! {
    static CHILD_INDEX: RefCell<ChildIndexCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(ChildIndexCore::new(BTreeMap::init(icu_register_memory!(
            CHILD_INDEX_MEMORY_ID
        ))));
}

///
/// ChildIndexError
///

#[derive(Debug, ThisError)]
pub enum ChildIndexError {
    #[error("canister not found: {0}")]
    CanisterNotFound(Principal),
}

///
/// ChildIndex
///

pub struct ChildIndex {}

impl ChildIndex {
    #[must_use]
    pub fn is_empty() -> bool {
        CHILD_INDEX.with_borrow(ChildIndexCore::is_empty)
    }

    #[must_use]
    pub fn get(pid: &Principal) -> Option<String> {
        CHILD_INDEX.with_borrow(|core| core.get(pid))
    }

    pub fn try_get(pid: &Principal) -> Result<String, Error> {
        CHILD_INDEX.with_borrow(|core| core.try_get(pid))
    }

    #[must_use]
    pub fn get_by_kind(kind: &str) -> Vec<Principal> {
        CHILD_INDEX.with_borrow(|core| core.get_by_kind(kind))
    }

    pub fn insert(pid: Principal, kind: &str) {
        CHILD_INDEX.with_borrow_mut(|core| {
            core.insert(pid, kind.to_string());
        });
    }

    pub fn remove(pid: &Principal) {
        CHILD_INDEX.with_borrow_mut(|core| {
            core.remove(pid);
        });
    }

    pub fn clear() {
        CHILD_INDEX.with_borrow_mut(|core| {
            core.clear();
        });
    }

    #[must_use]
    pub fn export() -> ChildIndexView {
        CHILD_INDEX.with_borrow(ChildIndexCore::export)
    }
}

///
/// ChildIndexCore
///

pub type ChildIndexView = HashMap<Principal, String>;

pub struct ChildIndexCore<M: Memory> {
    map: BTreeMap<Principal, String, M>,
}

impl<M: Memory> ChildIndexCore<M> {
    pub const fn new(map: BTreeMap<Principal, String, M>) -> Self {
        Self { map }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, pid: &Principal) -> Option<String> {
        self.map.get(pid)
    }

    #[must_use]
    pub fn get_by_kind(&self, kind: &str) -> Vec<Principal> {
        self.map
            .iter_pairs()
            .filter_map(|(p, k)| if k == kind { Some(p) } else { None })
            .collect()
    }

    pub fn try_get(&self, pid: &Principal) -> Result<String, Error> {
        if let Some(kind) = self.get(pid) {
            Ok(kind)
        } else {
            Err(MemoryError::from(ChildIndexError::CanisterNotFound(*pid)))?
        }
    }

    pub fn insert(&mut self, pid: Principal, kind: String) {
        self.map.insert(pid, kind);
    }

    pub fn remove(&mut self, pid: &Principal) {
        self.map.remove(pid);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn export(&self) -> ChildIndexView {
        self.map.iter_pairs().collect()
    }
}

impl<M: Memory> IntoIterator for ChildIndexCore<M> {
    type Item = (Principal, String);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_pairs().collect::<Vec<_>>().into_iter()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ic::structures::DefaultMemoryImpl;

    fn make_core() -> ChildIndexCore<DefaultMemoryImpl> {
        let map = BTreeMap::init(DefaultMemoryImpl::default());
        ChildIndexCore::new(map)
    }

    #[test]
    fn test_insert_and_get() {
        let mut core = make_core();
        let pid = Principal::anonymous();

        assert!(core.is_empty());

        core.insert(pid, "worker".to_string());

        assert!(!core.is_empty());
        assert_eq!(core.get(&pid), Some("worker".to_string()));
        assert_eq!(core.try_get(&pid).unwrap(), "worker".to_string());
    }

    #[test]
    fn test_get_by_kind() {
        let mut core = make_core();
        let p1 = Principal::from_slice(&[1]);
        let p2 = Principal::from_slice(&[2]);
        let p3 = Principal::from_slice(&[3]);

        core.insert(p1, "alpha".to_string());
        core.insert(p2, "beta".to_string());
        core.insert(p3, "alpha".to_string());

        let alphas = core.get_by_kind("alpha");
        assert!(alphas.contains(&p1));
        assert!(alphas.contains(&p3));
        assert_eq!(alphas.len(), 2);

        let betas = core.get_by_kind("beta");
        assert_eq!(betas, vec![p2]);
    }

    #[test]
    fn test_remove_and_clear() {
        let mut core = make_core();
        let pid = Principal::from_slice(&[42]);

        core.insert(pid, "gamma".to_string());
        assert_eq!(core.get(&pid), Some("gamma".to_string()));

        core.remove(&pid);
        assert_eq!(core.get(&pid), None);

        core.insert(pid, "gamma".to_string());
        assert!(!core.is_empty());
        core.clear();
        assert!(core.is_empty());
    }

    #[test]
    fn test_export_and_iter() {
        let mut core = make_core();
        let p1 = Principal::from_slice(&[1]);
        let p2 = Principal::from_slice(&[2]);

        core.insert(p1, "x".to_string());
        core.insert(p2, "y".to_string());

        let exported = core.export();
        assert_eq!(exported.get(&p1), Some(&"x".to_string()));
        assert_eq!(exported.get(&p2), Some(&"y".to_string()));

        // check IntoIterator impl
        let pairs: Vec<_> = core.into_iter().collect();
        assert!(pairs.contains(&(p1, "x".to_string())));
        assert!(pairs.contains(&(p2, "y".to_string())));
    }

    #[test]
    #[should_panic(expected = "canister not found")]
    fn test_try_get_not_found() {
        let core = make_core();
        let pid = Principal::from_slice(&[9]);
        // this should trigger the ChildIndexError
        let _ = core.try_get(&pid).unwrap();
    }
}
