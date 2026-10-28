//! Exercise the adapter's host-memory binding with a framework-owned participant.
//! Actual database semantics and generated actor acceptance remain downstream.

pub(crate) mod __icydb_facade {
    pub(crate) mod db {
        use ic_memory::{DefaultMemoryImpl, MemoryRuntime};
        use std::{cell::RefCell, rc::Rc};

        thread_local! {
            static BOUND: RefCell<Option<Rc<MemoryRuntime<DefaultMemoryImpl>>>> = const { RefCell::new(None) };
        }

        pub(crate) fn bind_memory_runtime(
            runtime: Rc<MemoryRuntime<DefaultMemoryImpl>>,
        ) -> Result<(), &'static str> {
            let host = canic::memory::runtime::shared_runtime().map_err(|_| "host not ready")?;
            assert!(Rc::ptr_eq(&host, &runtime));
            assert!(runtime.is_bootstrapped());
            BOUND.with_borrow_mut(|slot| *slot = Some(runtime));
            Ok(())
        }

        pub(crate) fn assert_bound() {
            BOUND.with_borrow(|slot| {
                assert!(Rc::ptr_eq(
                    slot.as_ref().expect("synchronous participant binding"),
                    &canic::memory::runtime::shared_runtime().unwrap()
                ));
            });
        }
    }
}

pub(crate) mod __icydb_host {
    pub fn __icydb_startup_init() {
        super::__icydb_facade::db::assert_bound();
        crate::lifecycle_participant_init();
    }

    pub fn __icydb_startup_post_upgrade() {
        super::__icydb_facade::db::assert_bound();
        crate::lifecycle_participant_post_upgrade();
    }
}

#[canic_icydb::__private::managed_entry(query, name = "adapter_runtime_binding", sdk = ic_platform::cdk, max_bytes = crate::timer_wire::REQUEST_LIMITS.bytes)]
fn adapter_runtime_binding() {
    if !ic_platform::cdk::api::is_controller(&ic_platform::cdk::api::msg_caller()) {
        ic_platform::cdk::api::msg_reject("controller required");
        return;
    }
    canic_icydb::serve_raw(
        crate::timer_wire::REQUEST_LIMITS.bytes,
        |bytes| {
            ic_wire::native::decode_request::<()>(
                crate::timer_wire::ADAPTER_BINDING,
                bytes,
                crate::timer_wire::REQUEST_LIMITS,
            )
        },
        |response| {
            ic_wire::native::encode_response(
                crate::timer_wire::ADAPTER_BINDING,
                &response,
                crate::timer_wire::RESPONSE_LIMITS,
            )
        },
        |()| {
            __icydb_facade::db::assert_bound();
            true
        },
    );
}
