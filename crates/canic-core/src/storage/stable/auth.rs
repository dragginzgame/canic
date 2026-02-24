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

    #[must_use]
    pub(crate) fn get_root_public_key() -> Option<Vec<u8>> {
        DELEGATION_STATE.with_borrow(|cell| cell.get().root_public_key.clone())
    }

    pub(crate) fn set_root_public_key(public_key_sec1: Vec<u8>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();
            data.root_public_key = Some(public_key_sec1);
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn get_shard_public_key(shard_pid: Principal) -> Option<Vec<u8>> {
        DELEGATION_STATE.with_borrow(|cell| {
            cell.get()
                .shard_public_keys
                .iter()
                .find(|entry| entry.shard_pid == shard_pid)
                .map(|entry| entry.public_key_sec1.clone())
        })
    }

    pub(crate) fn set_shard_public_key(shard_pid: Principal, public_key_sec1: Vec<u8>) {
        DELEGATION_STATE.with_borrow_mut(|cell| {
            let mut data = cell.get().clone();

            if let Some(entry) = data
                .shard_public_keys
                .iter_mut()
                .find(|entry| entry.shard_pid == shard_pid)
            {
                entry.public_key_sec1 = public_key_sec1;
            } else {
                data.shard_public_keys.push(ShardPublicKeyRecord {
                    shard_pid,
                    public_key_sec1,
                });
            }

            cell.set(data);
        });
    }
}

///
/// DelegationCertRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DelegationCertRecord {
    pub root_pid: Principal,
    pub shard_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub scopes: Vec<String>,
    pub aud: Vec<Principal>,
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
/// ShardPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShardPublicKeyRecord {
    pub shard_pid: Principal,
    pub public_key_sec1: Vec<u8>,
}

///
/// DelegationStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DelegationStateRecord {
    #[serde(default)]
    pub proof: Option<DelegationProofRecord>,

    #[serde(default)]
    pub root_public_key: Option<Vec<u8>>,

    #[serde(default)]
    pub shard_public_keys: Vec<ShardPublicKeyRecord>,
}

impl_storable_unbounded!(DelegationStateRecord);
