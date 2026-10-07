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

ic_memory::ic_memory_range!(
    authority = "managed-probe",
    start = 125,
    end = 125,
    mode = Allowed
);
ic_memory::ic_memory_declaration!(
    authority = "managed-probe",
    key = "managed_probe.initialization.v1"
);

thread_local! {
    static EVIDENCE: RefCell<Option<Evidence>> = const { RefCell::new(None) };
}

pub fn install() {
    let (_, args): (CanisterInitPayload, Option<Vec<u8>>) =
        LIMITS.read().expect("bounded init envelope");
    let memory =
        ic_memory::open_default_memory_manager_memory_by_key(KEY).expect("application grant");
    assert_eq!(memory.size(), 0);
    EVIDENCE.with_borrow_mut(|cell| *cell = Some(Cell::init(memory, args.unwrap_or_default())));
}

pub fn restore() {
    let memory = ic_memory::open_default_memory_manager_memory_by_key(KEY)
        .expect("retained application grant");
    assert!(memory.size() > 0);
    EVIDENCE.with_borrow_mut(|cell| *cell = Some(Cell::init(memory, Vec::new())));
}

pub fn bytes() -> Vec<u8> {
    EVIDENCE.with_borrow(|cell| cell.as_ref().expect("restored evidence").get().clone())
}
