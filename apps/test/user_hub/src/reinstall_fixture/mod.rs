//! Module: reinstall_fixture
//!
//! Responsibility: retain neutral application rows for destructive reset qualification.
//! Does not own: Canic lifecycle or Fleet orchestration.
//! Boundary: application memory is initialized only after Canic restoration.

use canic::__internal::core::cdk::structures::{DefaultMemoryImpl, StableBTreeMap as BTreeMap};
use std::cell::RefCell;

const ROWS_KEY: &str = "test.user_rows.v1";

canic::memory::memory_allocation_pool!(
    provider = memory_pool,
    authorities = [("test", "test.")],
    exclusions = []
);

pub fn memory() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.pool = memory_pool().expect("fixture grants");
    for key in [ROWS_KEY, super::fixture_importer::MEMORY_KEY] {
        input
            .requests
            .push(ic_memory::MemoryRequest::new("test", key).expect("fixture memory request"));
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
    static ROWS: RefCell<BTreeMap<u64, u64, DefaultMemoryImpl>> = RefCell::new(
        BTreeMap::init(canic::memory::open(ROWS_KEY))
    );
}

pub fn seed() {
    insert(0, 7);
}
pub fn insert(id: u64, value: u64) {
    ROWS.with_borrow_mut(|rows| {
        rows.insert(id, value);
    });
}
pub fn rows() -> Vec<UserRow> {
    ROWS.with_borrow(|rows| {
        rows.iter()
            .map(|entry| {
                let (id, value) = entry.into_pair();
                UserRow { id, value }
            })
            .collect()
    })
}
