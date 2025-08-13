use crate::{
    Error,
    ic::{api::canister_self, structures::Cell},
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
    pub static CANISTER_STATE: RefCell<Cell<CanisterStateData>> = RefCell::new(Cell::init(
        icu_register_memory!(CanisterStateData, CANISTER_STATE_MEMORY_ID),
        CanisterStateData::default(),
    ));
}

// ROOT_KIND is special

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
/// CanisterState
///

pub struct CanisterState {}

impl CanisterState {
    //
    // INTERNAL ACCESSORS
    //

    pub fn with<R>(f: impl FnOnce(&Cell<CanisterStateData>) -> R) -> R {
        CANISTER_STATE.with_borrow(|cell| f(cell))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut Cell<CanisterStateData>) -> R) -> R {
        CANISTER_STATE.with_borrow_mut(|cell| f(cell))
    }

    //
    // METHODS
    //

    #[must_use]
    pub fn get_kind() -> Option<String> {
        Self::with(|cell| cell.get().kind)
    }

    pub fn try_get_kind() -> Result<String, Error> {
        if let Some(kind) = Self::get_kind() {
            Ok(kind)
        } else {
            Err(MemoryError::from(CanisterStateError::KindNotSet))?
        }
    }

    #[must_use]
    pub fn is_root() -> bool {
        Self::get_parents().is_empty()
    }

    pub fn get_root_pid() -> Principal {
        Self::get_parents()
            .first()
            .map_or_else(canister_self, |p| p.principal)
    }

    pub fn set_kind(kind: &str) -> Result<(), Error> {
        if kind == ROOT_KIND {
            Err(MemoryError::from(CanisterStateError::KindReserved))?;
        }

        Self::with_mut(|cell| {
            let mut state = cell.get();
            state.kind = Some(kind.to_string());
            cell.set(state)
        });

        Ok(())
    }

    pub fn set_kind_root() {
        Self::with_mut(|cell| {
            let mut state = cell.get();
            state.kind = Some(ROOT_KIND.to_string());
            cell.set(state)
        });
    }

    #[must_use]
    pub fn get_parents() -> Vec<CanisterParent> {
        Self::with(|cell| cell.get().parents)
    }

    #[must_use]
    pub fn get_parent_by_kind(kind: &str) -> Option<Principal> {
        Self::get_parents()
            .iter()
            .find(|p| p.kind == kind)
            .map(|p| p.principal)
    }

    #[must_use]
    pub fn has_parent_pid(parent_pid: &Principal) -> bool {
        Self::get_parents()
            .iter()
            .any(|p| p.principal == *parent_pid)
    }

    pub fn set_parents(parents: Vec<CanisterParent>) {
        Self::with_mut(|cell| {
            let mut state = cell.get();
            state.parents = parents;
            cell.set(state);
        });
    }

    //
    // EXPORT
    //

    #[must_use]
    pub fn export() -> CanisterStateData {
        Self::with(Cell::get)
    }
}

///
/// CanisterStateData
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CanisterStateData {
    kind: Option<String>,
    parents: Vec<CanisterParent>,
}

impl_storable_unbounded!(CanisterStateData);

///
/// CanisterParent
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
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
