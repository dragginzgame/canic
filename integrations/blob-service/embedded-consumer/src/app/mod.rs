//! Own the example application's counter independently of blob state.

use std::cell::RefCell;

use canic::dto::public_status::{PublicMetric, PublicMetricKind};
use ic_memory::{
    RuntimeMemory,
    ic_stable_structures::{Cell, DefaultMemoryImpl, Memory},
};

const KEY: &str = "embedded_app.counter.v1";
type Counter = Cell<u64, RuntimeMemory<DefaultMemoryImpl>>;

ic_memory::ic_memory_declaration!(authority = "embedded-app", key = "embedded_app.counter.v1");

thread_local! {
    static COUNTER: RefCell<Option<Counter>> = const { RefCell::new(None) };
}

pub(super) fn install(initial: u64) {
    let memory = ic_memory::open_default_memory_manager_memory(KEY).expect("counter grant");
    assert_eq!(memory.size(), 0, "fresh counter memory");
    COUNTER.with_borrow_mut(|counter| *counter = Some(Cell::init(memory, initial)));
}

pub(super) fn restore() {
    let memory = ic_memory::open_default_memory_manager_memory(KEY).expect("counter grant");
    assert!(memory.size() > 0, "retained counter memory");
    COUNTER.with_borrow_mut(|counter| *counter = Some(Cell::init(memory, 0)));
}

pub(super) fn count() -> u64 {
    COUNTER.with_borrow(|counter| *counter.as_ref().expect("initialized counter").get())
}

pub(super) fn increment() -> u64 {
    COUNTER.with_borrow_mut(|counter| {
        let counter = counter.as_mut().expect("initialized counter");
        let next = counter.get().saturating_add(1);
        counter.set(next);
        next
    })
}

pub(super) fn metric() -> PublicMetric {
    PublicMetric {
        name: "app.count".into(),
        canister_id: Some(ic_cdk::api::canister_self()),
        value: u128::from(count()),
        unit: "count".into(),
        observed_at_ns: ic_cdk::api::time(),
        kind: PublicMetricKind::Gauge,
    }
}
