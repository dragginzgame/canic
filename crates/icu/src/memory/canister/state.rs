use crate::{
    Error,
    canister::CanisterType,
    ic::structures::{Cell, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_STATE_MEMORY_ID, MemoryError, canister::CanisterEntry},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// CANISTER_STATE
//

thread_local! {
    pub static CANISTER_STATE: RefCell<CanisterStateCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(CanisterStateCore::new(Cell::init(
            icu_register_memory!(CANISTER_STATE_MEMORY_ID),
            CanisterStateData::default(),
        )));
}

///
/// CanisterStateError
///

#[derive(Debug, ThisError)]
pub enum CanisterStateError {
    #[error("canister type has not been set")]
    TypeNotSet,

    #[error("this canister does not have any parents")]
    NoParents,
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterStateData {
    pub canister_type: Option<CanisterType>,
    pub parents: Vec<CanisterEntry>,
}

impl_storable_unbounded!(CanisterStateData);

///
/// CanisterState
///

pub struct CanisterState;

impl CanisterState {
    #[must_use]
    pub fn get_type() -> Option<CanisterType> {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_type)
    }

    pub fn try_get_type() -> Result<CanisterType, Error> {
        CANISTER_STATE.with_borrow(CanisterStateCore::try_get_type)
    }

    #[must_use]
    pub fn is_root() -> bool {
        CANISTER_STATE.with_borrow(CanisterStateCore::is_root)
    }

    #[must_use]
    pub fn get_root_pid() -> Principal {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_root_pid)
    }

    pub fn set_type(ty: &CanisterType) -> Result<(), Error> {
        CANISTER_STATE.with_borrow_mut(|core| core.set_type(ty))
    }

    #[must_use]
    pub fn get_parents() -> Vec<CanisterEntry> {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_parents)
    }

    #[must_use]
    pub fn get_parent_by_type(ty: &CanisterType) -> Option<Principal> {
        CANISTER_STATE.with_borrow(|core| core.get_parent_by_type(ty))
    }

    #[must_use]
    pub fn has_parent_pid(parent_pid: &Principal) -> bool {
        CANISTER_STATE.with_borrow(|core| core.has_parent_pid(parent_pid))
    }

    pub fn set_parents(parents: Vec<CanisterEntry>) {
        CANISTER_STATE.with_borrow_mut(|core| core.set_parents(parents));
    }

    #[must_use]
    pub fn export() -> CanisterStateData {
        CANISTER_STATE.with_borrow(CanisterStateCore::export)
    }

    pub fn import(data: CanisterStateData) {
        CANISTER_STATE.with_borrow_mut(|core| core.import(data));
    }
}

///
/// CanisterStateCore
///

pub struct CanisterStateCore<M: Memory> {
    cell: Cell<CanisterStateData, M>,
}

impl<M: Memory> CanisterStateCore<M> {
    pub const fn new(cell: Cell<CanisterStateData, M>) -> Self {
        Self { cell }
    }

    pub fn get_type(&self) -> Option<CanisterType> {
        self.cell.get().canister_type.clone()
    }

    pub fn try_get_type(&self) -> Result<CanisterType, Error> {
        self.get_type()
            .ok_or_else(|| MemoryError::from(CanisterStateError::TypeNotSet).into())
    }

    pub fn is_root(&self) -> bool {
        self.get_parents().is_empty()
    }

    pub fn get_root_pid(&self) -> Principal {
        self.get_parents()
            .first()
            .map_or_else(self_principal, |p| p.principal)
    }

    pub fn set_type(&mut self, ty: &CanisterType) -> Result<(), Error> {
        let mut state = self.cell.get().clone();
        state.canister_type = Some(ty.clone());
        self.cell.set(state);

        Ok(())
    }

    pub fn get_parents(&self) -> Vec<CanisterEntry> {
        self.cell.get().parents.clone()
    }

    pub fn get_parent_by_type(&self, ty: &CanisterType) -> Option<Principal> {
        self.get_parents()
            .iter()
            .find(|p| p.canister_type == *ty)
            .map(|p| p.principal)
    }

    pub fn has_parent_pid(&self, parent_pid: &Principal) -> bool {
        self.get_parents()
            .iter()
            .any(|p| &p.principal == parent_pid)
    }

    pub fn set_parents(&mut self, parents: Vec<CanisterEntry>) {
        let mut state = self.cell.get().clone();
        state.parents = parents;
        self.cell.set(state);
    }

    pub fn export(&self) -> CanisterStateData {
        self.cell.get().clone()
    }

    pub fn import(&mut self, data: CanisterStateData) {
        self.cell.set(data);
    }
}

#[allow(clippy::missing_const_for_fn)]
fn self_principal() -> Principal {
    #[cfg(test)]
    {
        Principal::anonymous() // or any dummy principal
    }

    #[cfg(not(test))]
    {
        crate::ic::api::canister_self()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ic::structures::DefaultMemoryImpl;

    fn make_core() -> CanisterStateCore<DefaultMemoryImpl> {
        let cell = Cell::init(DefaultMemoryImpl::default(), CanisterStateData::default());
        CanisterStateCore::new(cell)
    }

    #[test]
    fn test_default_type_is_none() {
        let core = make_core();
        assert_eq!(core.get_type(), None);
        assert!(core.try_get_type().is_err());
    }

    #[test]
    fn test_set_type_and_get() {
        let mut core = make_core();
        core.set_type(&CanisterType::new("worker")).unwrap();
        assert_eq!(core.get_type(), Some(CanisterType::new("worker")));
        assert_eq!(core.try_get_type().unwrap(), CanisterType::new("worker"));
    }

    #[test]
    fn test_is_root_and_get_root_pid() {
        let mut core = make_core();
        assert!(core.is_root());
        // no parents means fallback to self
        assert_eq!(core.get_root_pid(), self_principal());

        let parent = CanisterEntry {
            canister_type: CanisterType::new("foo"),
            principal: Principal::anonymous(),
        };
        core.set_parents(vec![parent.clone()]);
        assert!(!core.is_root());
        assert_eq!(core.get_root_pid(), parent.principal);
    }

    #[test]
    fn test_set_and_get_parents() {
        let mut core = make_core();
        let p1 = CanisterEntry {
            canister_type: CanisterType::new("alpha"),
            principal: Principal::anonymous(),
        };
        let p2 = CanisterEntry {
            canister_type: CanisterType::new("beta"),
            principal: Principal::management_canister(),
        };

        core.set_parents(vec![p1, p2]);
        let parents = core.get_parents();
        assert_eq!(parents.len(), 2);
        assert!(
            parents
                .iter()
                .any(|p| p.canister_type == CanisterType::new("alpha"))
        );
        assert!(
            parents
                .iter()
                .any(|p| p.canister_type == CanisterType::new("beta"))
        );
    }

    #[test]
    fn test_get_parent_by_type_and_has_parent_pid() {
        let mut core = make_core();
        let p1 = CanisterEntry {
            canister_type: CanisterType::new("alpha"),
            principal: Principal::anonymous(),
        };
        let p2 = CanisterEntry {
            canister_type: CanisterType::new("beta"),
            principal: Principal::management_canister(),
        };
        core.set_parents(vec![p1.clone(), p2.clone()]);

        assert_eq!(
            core.get_parent_by_type(&CanisterType::new("alpha")),
            Some(p1.principal)
        );
        assert_eq!(
            core.get_parent_by_type(&CanisterType::new("beta")),
            Some(p2.principal)
        );
        assert!(core.has_parent_pid(&p1.principal));
        assert!(core.has_parent_pid(&p2.principal));
        assert!(!core.has_parent_pid(&Principal::from_slice(&[42; 29])));
    }

    #[test]
    fn test_export_and_import() {
        let mut core = make_core();
        core.set_type(&CanisterType::new("worker")).unwrap();
        let parent = CanisterEntry {
            canister_type: CanisterType::new("p"),
            principal: Principal::anonymous(),
        };
        core.set_parents(vec![parent.clone()]);

        let exported = core.export();
        let mut new_core = make_core();
        new_core.import(exported);

        assert_eq!(new_core.get_type(), Some(CanisterType::new("worker")));
        assert_eq!(new_core.get_parents(), vec![parent]);
    }
}
