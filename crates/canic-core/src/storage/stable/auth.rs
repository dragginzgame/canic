use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    dto::auth::DelegationProof,
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{prelude::*, stable::memory::auth::DELEGATION_STATE_ID},
};
use std::cell::RefCell;

eager_static! {
    static DELEGATION_STATE: RefCell<Cell<DelegationStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(DelegationState, DELEGATION_STATE_ID),
            DelegationStateData::default(),
        ));
}

///
/// DelegationState
///

pub struct DelegationState;

impl DelegationState {
    #[must_use]
    pub(crate) fn export() -> DelegationStateData {
        DELEGATION_STATE.with_borrow(|cell| cell.get().clone())
    }

    pub(crate) fn import(data: DelegationStateData) {
        DELEGATION_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub(crate) fn get_proof() -> Option<DelegationProof> {
        DELEGATION_STATE.with_borrow(|cell| cell.get().proof.clone())
    }

    pub(crate) fn set_proof(proof: DelegationProof) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proof = Some(proof);
            cell.set(data);
        });
    }

    pub(crate) fn clear_proof() {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proof = None;
            cell.set(data);
        });
    }
}

///
/// DelegationStateData
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DelegationStateData {
    pub proof: Option<DelegationProof>,
}

impl_storable_unbounded!(DelegationStateData);
