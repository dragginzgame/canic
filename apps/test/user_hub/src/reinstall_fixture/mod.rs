//! Module: reinstall_fixture
//!
//! Responsibility: retain neutral application rows for destructive reset qualification.
//! Does not own: Canic lifecycle or Fleet orchestration.
//! Boundary: application memory is initialized only after Canic restoration.

use ic_stable_structures::{BTreeMap, DefaultMemoryImpl, memory_manager::VirtualMemory};
use std::cell::RefCell;

canic::memory::ic_memory_range!(authority = "test", start = 200, end = 200, mode = Allowed);

struct UserRows;

/// One passive application row exposed by the disposable fixture.
#[derive(candid::CandidType)]
pub struct UserRow {
    pub id: u64,
    pub value: u64,
}

thread_local! {
    static ROWS: RefCell<BTreeMap<u64, u64, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        BTreeMap::init(canic::memory::ic_memory_key!(authority = "test", key = "test.user_rows.v1", ty = UserRows, id = 200))
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
