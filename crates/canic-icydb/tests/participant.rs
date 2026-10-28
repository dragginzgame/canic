//! Framework-owned participant ordering and exact-once custody; no database dependency.

use std::cell::Cell;

thread_local! {
    static BOUND: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static REJECT_MEMORY: Cell<bool> = const { Cell::new(false) };
}

mod __icydb_facade {
    pub mod db {
        pub fn bind_memory_runtime(runtime: u8) -> Result<(), &'static str> {
            if runtime != 17 {
                return Err("unexpected runtime");
            }
            crate::BOUND.set(true);
            Ok(())
        }
    }
}

fn runtime() -> Result<u8, &'static str> {
    if REJECT_MEMORY.get() {
        Err("host not committed")
    } else {
        Ok(17)
    }
}

mod __icydb_host {
    pub fn __icydb_startup_init() {
        assert!(crate::BOUND.get());
        crate::CALLS.set(crate::CALLS.get() + 1);
    }
    pub fn __icydb_startup_post_upgrade() {
        assert!(crate::BOUND.get());
        crate::CALLS.set(crate::CALLS.get() + 1);
    }
}

canic_icydb::__icydb_start_participant_lifecycle!(crate::runtime);

#[test]
fn init_binds_before_startup_and_duplicate_participation_is_effect_free() {
    __icydb_lifecycle_participant::init();
    __icydb_lifecycle_participant::init();
    __icydb_lifecycle_participant::post_upgrade();
    assert_eq!(CALLS.get(), 1);
}

#[test]
fn post_upgrade_binds_before_startup_on_a_fresh_heap() {
    __icydb_lifecycle_participant::post_upgrade();
    assert!(BOUND.get());
    assert_eq!(CALLS.get(), 1);
}

#[test]
fn failed_host_binding_runs_no_startup_and_can_retry() {
    REJECT_MEMORY.set(true);
    assert!(std::panic::catch_unwind(__icydb_lifecycle_participant::init).is_err());
    assert_eq!(CALLS.get(), 0);
    assert!(!BOUND.get());
    REJECT_MEMORY.set(false);
    __icydb_lifecycle_participant::init();
    assert_eq!(CALLS.get(), 1);
}
