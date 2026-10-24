//! Exercise the generated participant boundary with explicit facade substitutes.
//! Core's runtime and the downstream database retain their separate qualification.

use std::{cell::RefCell, rc::Rc};

struct Runtime;

thread_local! {
    static HOST: RefCell<Option<Rc<Runtime>>> = const { RefCell::new(None) };
    static BOUND: RefCell<Option<Rc<Runtime>>> = const { RefCell::new(None) };
    static STARTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn runtime() -> Result<Rc<Runtime>, &'static str> {
    HOST.with_borrow(Clone::clone).ok_or("host not committed")
}

mod __icydb_facade {
    pub mod db {
        use crate::{BOUND, Runtime};
        use std::rc::Rc;

        pub fn bind_memory_runtime(runtime: Rc<Runtime>) -> Result<(), &'static str> {
            BOUND.with_borrow_mut(|bound| {
                if let Some(existing) = bound {
                    if Rc::ptr_eq(existing, &runtime) {
                        Ok(())
                    } else {
                        Err("already bound")
                    }
                } else {
                    *bound = Some(runtime);
                    Ok(())
                }
            })
        }
    }
}

mod __icydb_host {
    pub fn __icydb_startup_init() {
        crate::BOUND.with_borrow(|bound| {
            assert!(std::rc::Rc::ptr_eq(
                bound.as_ref().unwrap(),
                &crate::runtime().unwrap()
            ));
        });
        crate::STARTS.set(crate::STARTS.get() + 1);
    }
    pub fn __icydb_startup_post_upgrade() {
        __icydb_startup_init();
    }
}

canic_icydb::__icydb_start_participant_lifecycle!(crate::runtime);

#[test]
fn binds_exact_host_before_startup_and_does_not_replay_completed_work() {
    HOST.set(Some(Rc::new(Runtime)));
    __icydb_lifecycle_participant::init();
    __icydb_lifecycle_participant::post_upgrade();
    assert_eq!(STARTS.get(), 1);
}

#[test]
fn missing_host_refuses_before_startup_and_native_retry_releases_latch() {
    assert!(std::panic::catch_unwind(__icydb_lifecycle_participant::init).is_err());
    assert_eq!(STARTS.get(), 0);
    HOST.set(Some(Rc::new(Runtime)));
    __icydb_lifecycle_participant::init();
    assert_eq!(STARTS.get(), 1);
}

#[test]
fn foreign_binding_refuses_without_running_the_database() {
    let existing = Rc::new(Runtime);
    BOUND.set(Some(Rc::clone(&existing)));
    HOST.set(Some(Rc::new(Runtime)));
    assert!(std::panic::catch_unwind(__icydb_lifecycle_participant::init).is_err());
    assert_eq!(STARTS.get(), 0);
    HOST.set(Some(existing));
    __icydb_lifecycle_participant::init();
    assert_eq!(STARTS.get(), 1);
}
