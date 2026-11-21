//! A miner retains its last round and its attempt receipts across code upgrades.
mod endpoints;

use canic::{method::LIMITS, topology::Install};
use ic_storage::{DefaultMemoryImpl, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const PARENT: &str = "miner.parent.v1";
const WORK: &str = "miner.work.v3";

thread_local! {
    static WORK_CELL: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) };
}

#[derive(Deserialize, Serialize)]
struct Work {
    generation: ic_wire::Bytes<32>,
    round: u64,
    batches: u8,
    attempts: u64,
    finds: u8,
}

impl Work {
    fn encode(&self) -> Vec<u8> {
        ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS).expect("encode work")
    }

    fn decode(cell: &StableCell<Vec<u8>>) -> Self {
        ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, LIMITS).expect("open work")
    }
}

fn install(binding: Install) {
    assert!(
        binding.role == "miner" && binding.application.len() == 32,
        "invalid miner install"
    );
    open(Some(binding));
}

fn reopen() {
    open(None);
}

fn open(binding: Option<Install>) {
    let runtime = canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        binding.is_some(),
        "miner",
        &[PARENT, WORK],
        8,
    );
    canic::topology::attach_parent(&runtime, PARENT, binding.as_ref());
    let cell = canic::storage::cell(
        &runtime,
        WORK,
        LIMITS.bytes,
        binding.map(|v| {
            Work {
                generation: ic_wire::Bytes::new(v.application).expect("worker generation"),
                round: 0,
                batches: 0,
                attempts: 0,
                finds: 0,
            }
            .encode()
        }),
    );
    let work = Work::decode(&cell);
    assert!(
        work.batches <= 2
            && (work.round == 0) == (work.batches == 0)
            && work.attempts >= u64::from(work.batches)
            && work.finds < (1 << work.batches),
        "invalid miner record"
    );
    WORK_CELL.with_borrow_mut(|slot| *slot = Some(cell));
}
