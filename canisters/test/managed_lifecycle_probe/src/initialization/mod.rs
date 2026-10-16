//! Own the generic fixture's initialization evidence in application memory.

use canic::{dto::abi::v1::CanisterInitPayload, endpoint::ArgumentLimits};
use ic_memory::{
    RuntimeMemory,
    ic_stable_structures::{Cell, DefaultMemoryImpl, Memory},
};
use std::cell::RefCell;

type Evidence = Cell<Vec<u8>, RuntimeMemory<DefaultMemoryImpl>>;
const KEY: &str = "managed_probe.initialization.v1";
pub const LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 65_536,
    decoding_quota: 2_000_000,
    skipping_quota: 1024,
    max_type_len: 512,
    max_header_len: 16_384,
};

canic::memory::memory_allocation_pool!(
    provider = memory_pool,
    authorities = [("managed-probe", "managed_probe.")],
    exclusions = []
);
pub fn memory() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.pool = memory_pool().expect("fixture grants");
    input.requests.push(
        ic_memory::MemoryRequest::new("managed-probe", KEY, ic_memory::SchemaMetadata::default())
            .unwrap(),
    );
    input.requests.push(
        ic_memory::MemoryRequest::new(
            "managed-probe",
            super::checkpoint::KEY,
            ic_memory::SchemaMetadata::default(),
        )
        .unwrap(),
    );
    input
}

thread_local! {
    static EVIDENCE: RefCell<Option<Evidence>> = const { RefCell::new(None) };
}

pub fn install() {
    super::checkpoint::open();
    let (_, args): (CanisterInitPayload, Option<Vec<u8>>) =
        LIMITS.read().expect("bounded init envelope");
    let memory = canic::memory::open(KEY);
    assert_eq!(memory.size(), 0);
    EVIDENCE.with_borrow_mut(|cell| *cell = Some(Cell::init(memory, args.unwrap_or_default())));
}

pub fn restore() {
    super::checkpoint::open();
    let memory = canic::memory::open(KEY);
    assert!(memory.size() > 0);
    EVIDENCE.with_borrow_mut(|cell| *cell = Some(Cell::init(memory, Vec::new())));
}

pub fn bytes() -> Vec<u8> {
    EVIDENCE.with_borrow(|cell| cell.as_ref().expect("restored evidence").get().clone())
}
