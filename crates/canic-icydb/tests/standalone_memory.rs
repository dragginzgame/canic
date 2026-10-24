//! Standalone exports explicitly bootstrap before scheduling database startup.

use canic_icydb::__private::cdk as __icydb_host_sdk;

thread_local! {
    static BOOTSTRAPPED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static STARTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

const fn icydb_memory_pool() -> u8 {
    7
}

fn bootstrap_standalone_memory(pool: impl FnOnce() -> u8) -> Result<(), &'static str> {
    if pool() != 7 {
        return Err("pool refused");
    }
    BOOTSTRAPPED.set(true);
    Ok(())
}

mod __icydb_host {
    pub fn __icydb_startup_init() {
        assert!(crate::BOOTSTRAPPED.get());
        crate::STARTS.set(crate::STARTS.get() + 1);
    }
    pub fn __icydb_startup_post_upgrade() {
        __icydb_startup_init();
    }
}

canic_icydb::__icydb_start_lifecycle!();

#[test]
fn init_bootstraps_before_startup() {
    __icydb_startup_init();
    assert_eq!(STARTS.get(), 1);
}

#[test]
fn upgrade_bootstraps_before_startup() {
    __icydb_startup_post_upgrade();
    assert_eq!(STARTS.get(), 1);
}
