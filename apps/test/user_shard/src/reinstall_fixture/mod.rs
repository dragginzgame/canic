//! Module: reinstall_fixture
//!
//! Responsibility: retain neutral application rows for destructive reset qualification.
//! Does not own: Canic lifecycle or Fleet orchestration.
//! Boundary: application memory is initialized only after Canic restoration.

use canic::__internal::core::cdk::structures::{DefaultMemoryImpl, StableBTreeMap as BTreeMap};
use std::cell::RefCell;

const ROWS_KEY: &str = "test.user_rows.v1";

canic::memory::memory_allocation_grants!(
    provider = memory_grants,
    authorities = [("test", "test.")],
);

pub fn memory() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.grants = memory_grants().expect("fixture grants");
    for key in [ROWS_KEY, super::fixture_importer::MEMORY_KEY] {
        input
            .requests
            .push(ic_storage::MemoryRequest::new("test", key).expect("fixture memory request"));
    }
    input
}

/// One passive application row exposed by the disposable fixture.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct UserRow {
    pub id: u64,
    pub value: u64,
}

thread_local! {
    static ROWS: RefCell<BTreeMap<u64, u64, DefaultMemoryImpl>> = RefCell::new({
        let memory = canic::memory::open(ROWS_KEY);
        if ic_storage::Memory::size(&memory) == 0 {
            BTreeMap::create(memory).expect("open fixture collection")
        } else {
            BTreeMap::load(memory).expect("open fixture collection")
        }
    });
}

pub fn seed() {
    insert(0, 7);
}
pub fn insert(id: u64, value: u64) {
    ROWS.with_borrow_mut(|rows| {
        rows.insert(id, value).expect("persist application row");
    });
}
pub fn rows() -> Vec<UserRow> {
    ROWS.with_borrow(|rows| {
        rows.iter()
            .map(|entry| entry.expect("read application row"))
            .map(|entry| {
                let (id, value) = entry.into_pair().expect("decode application row");
                UserRow { id, value }
            })
            .collect()
    })
}
