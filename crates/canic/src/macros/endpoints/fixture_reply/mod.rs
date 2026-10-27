//! Internal-fixture-only response scheduling for canonical Store endpoints.
//! Holds replies after real authorization/storage work; owns no durable state.
//! Ordinary Store builds emit neither the barrier nor its controller endpoints.

#[doc(hidden)]
#[cfg(not(feature = "internal-test-fixtures"))]
#[macro_export]
macro_rules! __canic_fixture_reply_endpoint {
    ($(#[$attribute:meta])* fn $name:ident($($arguments:tt)*) -> $result:ty $body:block) => {
        $(#[$attribute])*
        fn $name($($arguments)*) -> $result $body
    };
}

#[doc(hidden)]
#[cfg(feature = "internal-test-fixtures")]
#[macro_export]
macro_rules! __canic_fixture_reply_endpoint {
    ($(#[$attribute:meta])* fn $name:ident($($arguments:tt)*) -> $result:ty $body:block) => {
        $(#[$attribute])*
        async fn $name($($arguments)*) -> $result $body
    };
}

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
        use ::canic::dto::fixture_reply::ReplyKind as __CanicFixtureReplyKind;

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

        fn __canic_decode_fixture_reply_arm(bytes: Vec<u8>) -> __CanicFixtureReplyKind {
            $crate::__internal::ic_wire::native::decode_request(::canic::protocol::fixture_reply::arm::METHOD_ID, &bytes, ::canic::protocol::fixture_reply::LIMITS)
                .unwrap_or_else(|error| $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()))
        }
        fn __canic_encode_fixture_reply_arm(value: bool) -> Vec<u8> {
            $crate::__internal::ic_wire::native::encode_response(::canic::protocol::fixture_reply::arm::METHOD_ID, &value, ::canic::protocol::fixture_reply::LIMITS)
                .unwrap_or_else(|error| $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()))
        }

        #[$crate::__internal::cdk::update(guard = "__canic_require_fixture_controller", decode_with = "__canic_decode_fixture_reply_arm", encode_with = "__canic_encode_fixture_reply_arm")]
        fn canic_test_fixture_reply_arm(kind: __CanicFixtureReplyKind) -> bool {
            __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| {
                if barrier.waiting != 0 { return false; }
                barrier.kind = Some(kind);
                barrier.released = false;
                true
            })
        }

        fn __canic_decode_fixture_reply_release(bytes: Vec<u8>) {
            $crate::__internal::ic_wire::native::decode_request(::canic::protocol::fixture_reply::release::METHOD_ID, &bytes, ::canic::protocol::fixture_reply::LIMITS)
                .unwrap_or_else(|error| $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()))
        }
        fn __canic_encode_fixture_reply_release() -> Vec<u8> {
            $crate::__internal::ic_wire::native::encode_response(::canic::protocol::fixture_reply::release::METHOD_ID, &(), ::canic::protocol::fixture_reply::LIMITS)
                .unwrap_or_else(|error| $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()))
        }

        #[$crate::__internal::cdk::update(guard = "__canic_require_fixture_controller", decode_with = "__canic_decode_fixture_reply_release", encode_with = "__canic_encode_fixture_reply_release")]
        fn canic_test_fixture_reply_release() {
            __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| barrier.released = true);
        }

        fn __canic_decode_fixture_reply_waiting(bytes: Vec<u8>) {
            $crate::__internal::ic_wire::native::decode_request(::canic::protocol::fixture_reply::waiting::METHOD_ID, &bytes, ::canic::protocol::fixture_reply::LIMITS)
                .unwrap_or_else(|error| $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()))
        }
        fn __canic_encode_fixture_reply_waiting(value: u32) -> Vec<u8> {
            $crate::__internal::ic_wire::native::encode_response(::canic::protocol::fixture_reply::waiting::METHOD_ID, &value, ::canic::protocol::fixture_reply::LIMITS)
                .unwrap_or_else(|error| $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()))
        }

        #[$crate::__internal::cdk::query(guard = "__canic_require_fixture_controller", decode_with = "__canic_decode_fixture_reply_waiting", encode_with = "__canic_encode_fixture_reply_waiting")]
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
                    $crate::__internal::ic_platform::principal::ExternalPrincipal::management_canister(), "raw_rand",
                ).with_args(()).expect("fixture round arguments")
                    .execute_candid::<Vec<u8>>().await.expect("fixture consensus round");
                if __CANIC_FIXTURE_REPLY_BARRIER.with_borrow(|barrier| barrier.released) {
                    __CANIC_FIXTURE_REPLY_BARRIER.with_borrow_mut(|barrier| barrier.waiting -= 1);
                    return;
                }
            }
            $crate::__internal::core::control_plane_support::ops::ic::IcOps::trap("fixture reply barrier exhausted");
        }
    };
}
