//! The standalone root admits memory before registering startup on both hooks.
use ic_platform::cdk as __icydb_host_sdk;
use std::cell::Cell;

thread_local! {
    static BOOTSTRAPS: Cell<u8> = const { Cell::new(0) };
    static STARTUPS: Cell<u8> = const { Cell::new(0) };
}

const fn icydb_memory_pool() {}

fn bootstrap_standalone_memory(pool: impl FnOnce()) -> Result<(), &'static str> {
    pool();
    crate::BOOTSTRAPS.with(|count| count.set(count.get() + 1));
    if crate::BOOTSTRAPS.with(Cell::get) == 1 {
        Ok(())
    } else {
        Err("already bootstrapped")
    }
}

mod __icydb_host {
    pub fn __icydb_startup_init() {
        crate::BOOTSTRAPS.with(|count| assert_eq!(count.get(), 1));
        crate::STARTUPS.with(|count| count.set(count.get() + 1));
    }
    pub fn __icydb_startup_post_upgrade() {
        __icydb_startup_init();
    }
}

canic_icydb::__icydb_start_lifecycle!();

#[test]
fn standalone_init_bootstraps_before_startup() {
    __icydb_startup_init();
    STARTUPS.with(|count| assert_eq!(count.get(), 1));
}

#[test]
fn standalone_upgrade_bootstraps_before_startup() {
    __icydb_startup_post_upgrade();
    STARTUPS.with(|count| assert_eq!(count.get(), 1));
}
