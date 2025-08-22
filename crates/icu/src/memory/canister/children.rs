use crate::{
    Error,
    canister::CanisterType,
    ic::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory,
    memory::{CANISTER_CHILDREN_MEMORY_ID, MemoryError},
};
use candid::Principal;
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error as ThisError;

//
// CANISTER_CHILDREN
//

thread_local! {
    static CANISTER_CHILDREN: RefCell<CanisterChildrenCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterChildrenCore::new(BTreeMap::init(icu_register_memory!(
            CANISTER_CHILDREN_MEMORY_ID
        ))));
}

///
/// CanisterChildrenError
///

#[derive(Debug, ThisError)]
pub enum CanisterChildrenError {
    #[error("canister not found: {0}")]
    CanisterNotFound(Principal),
}

///
/// CanisterChildren
///

pub struct CanisterChildren {}

impl CanisterChildren {
    #[must_use]
    pub fn is_empty() -> bool {
        CANISTER_CHILDREN.with_borrow(CanisterChildrenCore::is_empty)
    }

    #[must_use]
    pub fn get(pid: &Principal) -> Option<CanisterType> {
        CANISTER_CHILDREN.with_borrow(|core| core.get(pid))
    }

    pub fn try_get(pid: &Principal) -> Result<CanisterType, Error> {
        CANISTER_CHILDREN.with_borrow(|core| core.try_get(pid))
    }

    #[must_use]
    pub fn get_by_type(ty: &CanisterType) -> Vec<Principal> {
        CANISTER_CHILDREN.with_borrow(|core| core.get_by_type(ty))
    }

    pub fn insert(pid: Principal, ty: CanisterType) {
        CANISTER_CHILDREN.with_borrow_mut(|core| {
            core.insert(pid, ty);
        });
    }

    pub fn remove(pid: &Principal) {
        CANISTER_CHILDREN.with_borrow_mut(|core| {
            core.remove(pid);
        });
    }

    pub fn clear() {
        CANISTER_CHILDREN.with_borrow_mut(|core| {
            core.clear();
        });
    }

    #[must_use]
    pub fn export() -> CanisterChildrenView {
        CANISTER_CHILDREN.with_borrow(CanisterChildrenCore::export)
    }
}

///
/// CanisterChildrenCore
///

pub type CanisterChildrenView = HashMap<Principal, CanisterType>;

pub struct CanisterChildrenCore<M: Memory> {
    map: BTreeMap<Principal, CanisterType, M>,
}

impl<M: Memory> CanisterChildrenCore<M> {
    pub const fn new(map: BTreeMap<Principal, CanisterType, M>) -> Self {
        Self { map }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, pid: &Principal) -> Option<CanisterType> {
        self.map.get(pid)
    }

    #[must_use]
    pub fn get_by_type(&self, ty: &CanisterType) -> Vec<Principal> {
        self.map
            .iter_pairs()
            .filter_map(|(p, t)| if t == *ty { Some(p) } else { None })
            .collect()
    }

    pub fn try_get(&self, pid: &Principal) -> Result<CanisterType, Error> {
        if let Some(ty) = self.get(pid) {
            Ok(ty)
        } else {
            Err(MemoryError::from(CanisterChildrenError::CanisterNotFound(
                *pid,
            )))?
        }
    }

    pub fn insert(&mut self, pid: Principal, ty: CanisterType) {
        self.map.insert(pid, ty);
    }

    pub fn remove(&mut self, pid: &Principal) {
        self.map.remove(pid);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn export(&self) -> CanisterChildrenView {
        self.map.iter_pairs().collect()
    }
}

impl<M: Memory> IntoIterator for CanisterChildrenCore<M> {
    type Item = (Principal, CanisterType);
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

    fn make_core() -> CanisterChildrenCore<DefaultMemoryImpl> {
        let map = BTreeMap::init(DefaultMemoryImpl::default());
        CanisterChildrenCore::new(map)
    }

    #[test]
    fn test_insert_and_get() {
        let mut core = make_core();
        let pid = Principal::anonymous();

        assert!(core.is_empty());

        core.insert(pid, CanisterType::new("worker"));

        assert!(!core.is_empty());
        assert_eq!(core.get(&pid), Some(CanisterType::new("worker")));
        assert_eq!(core.try_get(&pid).unwrap(), CanisterType::new("worker"));
    }

    #[test]
    fn test_get_by_type() {
        let mut core = make_core();
        let p1 = Principal::from_slice(&[1]);
        let p2 = Principal::from_slice(&[2]);
        let p3 = Principal::from_slice(&[3]);

        core.insert(p1, CanisterType::new("alpha"));
        core.insert(p2, CanisterType::new("beta"));
        core.insert(p3, CanisterType::new("alpha"));

        let alphas = core.get_by_type(&CanisterType::new("alpha"));
        assert!(alphas.contains(&p1));
        assert!(alphas.contains(&p3));
        assert_eq!(alphas.len(), 2);

        let betas = core.get_by_type(&CanisterType::new("beta"));
        assert_eq!(betas, vec![p2]);
    }

    #[test]
    fn test_remove_and_clear() {
        let mut core = make_core();
        let pid = Principal::from_slice(&[42]);

        core.insert(pid, CanisterType::new("gamma"));
        assert_eq!(core.get(&pid), Some(CanisterType::new("gamma")));

        core.remove(&pid);
        assert_eq!(core.get(&pid), None);

        core.insert(pid, CanisterType::new("gamma"));
        assert!(!core.is_empty());
        core.clear();
        assert!(core.is_empty());
    }

    #[test]
    fn test_export_and_iter() {
        let mut core = make_core();
        let p1 = Principal::from_slice(&[1]);
        let p2 = Principal::from_slice(&[2]);

        core.insert(p1, CanisterType::new("x"));
        core.insert(p2, CanisterType::new("y"));

        let exported = core.export();
        assert_eq!(exported.get(&p1), Some(&CanisterType::new("x")));
        assert_eq!(exported.get(&p2), Some(&CanisterType::new("y")));

        // check IntoIterator impl
        let pairs: Vec<_> = core.into_iter().collect();
        assert!(pairs.contains(&(p1, CanisterType::new("x"))));
        assert!(pairs.contains(&(p2, CanisterType::new("y"))));
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
