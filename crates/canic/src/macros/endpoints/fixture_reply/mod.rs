//! Internal-fixture-only response scheduling for canonical Store endpoints.
//! Holds replies after real authorization/storage work; owns no durable state.
//! Ordinary Store builds emit neither the barrier nor its controller endpoints.

#[doc(hidden)]
#[cfg(not(feature = "internal-test-fixtures"))]
#[macro_export]
macro_rules! __canic_fixture_reply_support {
    () => {};
}

#[doc(hidden)]
#[cfg(not(feature = "internal-test-fixtures"))]
#[macro_export]
macro_rules! __canic_fixture_reply_checkpoint {
    ($kind:expr) => {};
}

#[doc(hidden)]
#[cfg(feature = "internal-test-fixtures")]
#[macro_export]
macro_rules! __canic_fixture_reply_checkpoint {
    ($kind:expr) => {
        __canic_hold_fixture_reply($kind).await;
    };
}

#[doc(hidden)]
#[cfg(feature = "internal-test-fixtures")]
#[macro_export]
macro_rules! __canic_fixture_reply_support {
    () => {
        #[derive(::canic::__internal::candid::CandidType, Clone, Copy, ::canic::__internal::serde::Deserialize, Eq, PartialEq)]
        #[serde(crate = "::canic::__internal::serde")]
        enum __CanicFixtureReplyKind { Grant, Revoke, Chunk }

        struct __CanicFixtureReplyBarrier {
            kind: Option<__CanicFixtureReplyKind>,
            waiting: u32,
            released: bool,
        }

        ::std::thread_local! {
            static __CANIC_FIXTURE_REPLY_BARRIER: ::std::cell::RefCell<__CanicFixtureReplyBarrier> = const {
                ::std::cell::RefCell::new(__CanicFixtureReplyBarrier {
                    kind: None, waiting: 0, released: false,
                })
            };
        }

        fn __canic_require_fixture_controller() -> Result<(), String> {
            if $crate::__internal::cdk::api::is_controller(&$crate::__internal::cdk::api::msg_caller()) {
                Ok(())
            } else {
                Err("fixture controller required".to_string())
            }
        }

        #[$crate::__internal::cdk::update(guard = "__canic_require_fixture_controller")]
        fn canic_test_fixture_reply_arm(kind: __CanicFixtureReplyKind) -> bool {
            __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| {
                if barrier.waiting != 0 { return false; }
                barrier.kind = Some(kind);
                barrier.released = false;
                true
            })
        }

        #[$crate::__internal::cdk::update(guard = "__canic_require_fixture_controller")]
        fn canic_test_fixture_reply_release() {
            __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| barrier.released = true);
        }

        #[$crate::__internal::cdk::query(guard = "__canic_require_fixture_controller")]
        fn canic_test_fixture_reply_waiting() -> u32 {
            __CANIC_FIXTURE_REPLY_BARRIER.with_borrow(|barrier| barrier.waiting)
        }

        async fn __canic_hold_fixture_reply(kind: __CanicFixtureReplyKind) {
            let held = __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| {
                if barrier.kind != Some(kind) || barrier.released { return false; }
                barrier.waiting = barrier.waiting.checked_add(1).expect("fixture waiting bound");
                true
            });
            if !held { return; }
            // Consensus calls expose real round boundaries while the original Store
            // response remains pending. Use an unbounded management response so a
            // caller-timeout clock jump cannot trap this test barrier itself.
            // The finite round cap prevents a hung qualification.
            for _ in 0..256 {
                $crate::prelude::Call::unbounded_wait(
                    $crate::__internal::candid::Principal::management_canister(), "raw_rand",
                ).with_args(()).expect("fixture round arguments")
                    .execute_candid::<Vec<u8>>().await.expect("fixture consensus round");
                if __CANIC_FIXTURE_REPLY_BARRIER.with_borrow(|barrier| barrier.released) {
                    __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| barrier.waiting -= 1);
                    return;
                }
            }
            $crate::__internal::cdk::trap("fixture reply barrier exhausted");
        }
    };
}
