use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{prelude::*, stable::memory::auth::DELEGATION_STATE_ID},
};
use std::cell::RefCell;

eager_static! {
    static DELEGATION_STATE: RefCell<Cell<DelegationStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(DelegationState, DELEGATION_STATE_ID),
            DelegationStateRecord::default(),
        ));
}

///
/// DelegationState
///

pub struct DelegationState;

impl DelegationState {
    #[must_use]
    pub(crate) fn get_proof() -> Option<DelegationProofRecord> {
        DELEGATION_STATE.with_borrow(|cell| cell.get().proof.clone())
    }

    pub(crate) fn set_proof(proof: DelegationProofRecord) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.proof = Some(proof);
            cell.set(data);
        });
    }
}

///
/// DelegationCertRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DelegationCertRecord {
    pub v: u16,
    pub signer_pid: Principal,
    pub audiences: Vec<String>,
    pub scopes: Vec<String>,
    pub issued_at: u64,
    pub expires_at: u64,
}

///
/// DelegationProofRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DelegationProofRecord {
    pub cert: DelegationCertRecord,
    pub cert_sig: Vec<u8>,
}

///
/// DelegationStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DelegationStateRecord {
    pub proof: Option<DelegationProofRecord>,
}

impl_storable_unbounded!(DelegationStateRecord);
