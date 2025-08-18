use crate::{
    Error,
    ic::{
        api::canister_self,
        structures::{Cell, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    },
    icu_register_memory, impl_storable_unbounded,
    memory::{CANISTER_STATE_MEMORY_ID, MemoryError},
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
/// Root Kind is special
///

pub const ROOT_KIND: &str = "root";

///
/// CanisterStateError
///

#[derive(Debug, ThisError)]
pub enum CanisterStateError {
    #[error("canister kind has not been set")]
    KindNotSet,

    #[error("this canister kind has been reserved")]
    KindReserved,

    #[error("this canister does not have any parents")]
    NoParents,
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterStateData {
    pub kind: Option<String>,
    pub parents: Vec<CanisterParent>,
}

impl_storable_unbounded!(CanisterStateData);

///
/// CanisterParent
///

#[derive(CandidType, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CanisterParent {
    pub kind: String,
    pub principal: Principal,
}

impl CanisterParent {
    pub fn this() -> Result<Self, Error> {
        let kind = CanisterState::try_get_kind()?;
        Ok(Self {
            kind,
            principal: canister_self(),
        })
    }
}

///
/// CanisterState
///

pub struct CanisterState;

impl CanisterState {
    #[must_use]
    pub fn get_kind() -> Option<String> {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_kind)
    }

    pub fn try_get_kind() -> Result<String, Error> {
        CANISTER_STATE.with_borrow(CanisterStateCore::try_get_kind)
    }

    #[must_use]
    pub fn is_root() -> bool {
        CANISTER_STATE.with_borrow(CanisterStateCore::is_root)
    }

    #[must_use]
    pub fn get_root_pid() -> Principal {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_root_pid)
    }

    pub fn set_kind(kind: &str) -> Result<(), Error> {
        CANISTER_STATE.with_borrow_mut(|core| core.set_kind(kind))
    }

    pub fn set_kind_root() {
        CANISTER_STATE.with_borrow_mut(CanisterStateCore::set_kind_root);
    }

    #[must_use]
    pub fn get_parents() -> Vec<CanisterParent> {
        CANISTER_STATE.with_borrow(CanisterStateCore::get_parents)
    }

    #[must_use]
    pub fn get_parent_by_kind(kind: &str) -> Option<Principal> {
        CANISTER_STATE.with_borrow(|core| core.get_parent_by_kind(kind))
    }

    #[must_use]
    pub fn has_parent_pid(parent_pid: &Principal) -> bool {
        CANISTER_STATE.with_borrow(|core| core.has_parent_pid(parent_pid))
    }

    pub fn set_parents(parents: Vec<CanisterParent>) {
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

    pub fn get_kind(&self) -> Option<String> {
        self.cell.get().kind.clone()
    }

    pub fn try_get_kind(&self) -> Result<String, Error> {
        self.get_kind()
            .ok_or_else(|| MemoryError::from(CanisterStateError::KindNotSet).into())
    }

    pub fn is_root(&self) -> bool {
        self.get_parents().is_empty()
    }

    pub fn get_root_pid(&self) -> Principal {
        self.get_parents()
            .first()
            .map_or_else(self_principal, |p| p.principal)
    }

    pub fn set_kind(&mut self, kind: &str) -> Result<(), Error> {
        if kind == ROOT_KIND {
            return Err(MemoryError::from(CanisterStateError::KindReserved))?;
        }

        let mut state = self.cell.get().clone();
        state.kind = Some(kind.to_string());
        self.cell.set(state);
        Ok(())
    }

    pub fn set_kind_root(&mut self) {
        let mut state = self.cell.get().clone();
        state.kind = Some(ROOT_KIND.to_string());
        self.cell.set(state);
    }

    pub fn get_parents(&self) -> Vec<CanisterParent> {
        self.cell.get().parents.clone()
    }

    pub fn get_parent_by_kind(&self, kind: &str) -> Option<Principal> {
        self.get_parents()
            .iter()
            .find(|p| p.kind == kind)
            .map(|p| p.principal)
    }

    pub fn has_parent_pid(&self, parent_pid: &Principal) -> bool {
        self.get_parents()
            .iter()
            .any(|p| &p.principal == parent_pid)
    }

    pub fn set_parents(&mut self, parents: Vec<CanisterParent>) {
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
        canister_self()
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
    fn test_default_kind_is_none() {
        let core = make_core();
        assert_eq!(core.get_kind(), None);
        assert!(core.try_get_kind().is_err());
    }

    #[test]
    fn test_set_kind_and_get() {
        let mut core = make_core();
        core.set_kind("worker").unwrap();
        assert_eq!(core.get_kind(), Some("worker".to_string()));
        assert_eq!(core.try_get_kind().unwrap(), "worker".to_string());
    }

    #[test]
    fn test_set_kind_reserved_fails() {
        let mut core = make_core();
        let err = core.set_kind(ROOT_KIND).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("reserved"));
    }

    #[test]
    fn test_set_kind_root() {
        let mut core = make_core();
        core.set_kind_root();
        assert_eq!(core.get_kind(), Some(ROOT_KIND.to_string()));
    }

    #[test]
    fn test_is_root_and_get_root_pid() {
        let mut core = make_core();
        assert!(core.is_root());
        // no parents means fallback to self
        assert_eq!(core.get_root_pid(), self_principal());

        let parent = CanisterParent {
            kind: "foo".to_string(),
            principal: Principal::anonymous(),
        };
        core.set_parents(vec![parent.clone()]);
        assert!(!core.is_root());
        assert_eq!(core.get_root_pid(), parent.principal);
    }

    #[test]
    fn test_set_and_get_parents() {
        let mut core = make_core();
        let p1 = CanisterParent {
            kind: "alpha".to_string(),
            principal: Principal::anonymous(),
        };
        let p2 = CanisterParent {
            kind: "beta".to_string(),
            principal: Principal::management_canister(),
        };

        core.set_parents(vec![p1, p2]);
        let parents = core.get_parents();
        assert_eq!(parents.len(), 2);
        assert!(parents.iter().any(|p| p.kind == "alpha"));
        assert!(parents.iter().any(|p| p.kind == "beta"));
    }

    #[test]
    fn test_get_parent_by_kind_and_has_parent_pid() {
        let mut core = make_core();
        let p1 = CanisterParent {
            kind: "alpha".to_string(),
            principal: Principal::anonymous(),
        };
        let p2 = CanisterParent {
            kind: "beta".to_string(),
            principal: Principal::management_canister(),
        };
        core.set_parents(vec![p1.clone(), p2.clone()]);

        assert_eq!(core.get_parent_by_kind("alpha"), Some(p1.principal));
        assert_eq!(core.get_parent_by_kind("beta"), Some(p2.principal));
        assert!(core.has_parent_pid(&p1.principal));
        assert!(core.has_parent_pid(&p2.principal));
        assert!(!core.has_parent_pid(&Principal::from_slice(&[42; 29])));
    }

    #[test]
    fn test_export_and_import() {
        let mut core = make_core();
        core.set_kind("worker").unwrap();
        let parent = CanisterParent {
            kind: "p".to_string(),
            principal: Principal::anonymous(),
        };
        core.set_parents(vec![parent.clone()]);

        let exported = core.export();
        let mut new_core = make_core();
        new_core.import(exported);

        assert_eq!(new_core.get_kind(), Some("worker".to_string()));
        assert_eq!(new_core.get_parents(), vec![parent]);
    }
}
