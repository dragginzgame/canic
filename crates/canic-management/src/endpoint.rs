//! Explicit management endpoint mounting. Platform owns execution; Wire owns codecs.

#[doc(hidden)]
pub use canic_management_protocol as protocol;
#[doc(hidden)]
pub use ic_platform_runtime::cdk;
#[doc(hidden)]
pub use ic_wire as wire;

/// Admission before Platform copies arguments. Authorization deliberately comes first.
///
/// # Errors
/// Refuses non-controllers on private methods, then oversized request frames.
#[doc(hidden)]
pub fn admit(limit: usize, private: bool) -> Result<(), String> {
    if private && !cdk::api::is_controller(&cdk::api::msg_caller()) {
        return Err("controller required".into());
    }
    if ic_platform_runtime::ic0::msg_arg_data_size() > limit {
        return Err("growth frame too large".into());
    }
    Ok(())
}

/// Response policy, with a deliberately impossible frame budget in fault builds.
#[doc(hidden)]
#[must_use]
pub fn response_limits(method: u64) -> wire::Limits {
    #[cfg(feature = "fault-injection")]
    if crate::engine::reply_fault(method) {
        return wire::Limits {
            bytes: 0,
            ..protocol::LIMITS
        };
    }
    #[cfg(not(feature = "fault-injection"))]
    let _ = method;
    protocol::LIMITS
}

/// Mount explicitly selected management methods using Platform's IC entrypoints.
///
/// Names are the existing exports: `status`, `observe`, `stage`, `reconcile`,
/// `stage_upgrade`, `upgrade_status`, `observe_upgrade`, `reconcile_upgrade`,
/// and (only with the supplier's `fault-injection` feature) `inject_fault`.
/// Duplicate selections fail compilation. Unknown names have no expansion.
/// This mounts no lifecycle hooks, storage, timers or readiness policy.
///
/// ```ignore
/// canic_management::mount_endpoints! {
///     status, observe, stage, reconcile,
///     stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
/// }
/// ```
#[macro_export]
macro_rules! mount_endpoints {
    ($($name:ident),+ $(,)?) => { $($crate::mount_endpoints!(@select $name);)+ };
    (@select status) => {
        $crate::mount_endpoints!(@method status, query, false, STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::Status,
            $crate::status(), "status frame", "bounded status");
    };
    (@select observe) => {
        $crate::mount_endpoints!(@method observe, update, true, STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::Status,
            $crate::status(), "status frame", "bounded status");
    };
    (@select stage) => {
        $crate::mount_endpoints!(@method stage, update, true, STAGE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::Stage)
                -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stage(input), "bounded stage", "bounded status");
    };
    (@select reconcile) => {
        $crate::mount_endpoints!(@method reconcile, update, true, RECONCILE, LIMITS,
            [async] (expected: $crate::endpoint::protocol::Approval) -> $crate::endpoint::protocol::Status,
            $crate::reconcile(expected).await, "reconcile frame", "bounded status");
    };
    (@select stage_upgrade) => {
        $crate::mount_endpoints!(@method stage_upgrade, update, true, upgrade::STAGE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::upgrade::Stage)
                -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stage_upgrade(input), "upgrade frame", "upgrade reply");
    };
    (@select upgrade_status) => {
        $crate::mount_endpoints!(@method upgrade_status, query, false, upgrade::STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::upgrade::Status,
            $crate::upgrade_status(), "upgrade status frame", "upgrade status reply");
    };
    (@select observe_upgrade) => {
        $crate::mount_endpoints!(@method observe_upgrade, update, true, upgrade::STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::upgrade::Status,
            $crate::upgrade_status(), "upgrade status frame", "upgrade status reply");
    };
    (@select reconcile_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_upgrade, update, true, upgrade::RECONCILE, LIMITS,
            [async] (expected: $crate::endpoint::protocol::upgrade::Approval) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::reconcile_upgrade(expected).await, "upgrade reconciliation frame", "upgrade reconciliation reply");
    };
    (@select inject_fault) => {
        $crate::mount_endpoints!(@method inject_fault, update, true, INJECT, LIMITS,
            [] (value: u8) -> (),
            $crate::inject_fault(value), "fault frame", "fault reply");
    };
    // One expansion for this actor's fixed endpoint surface, not a public generic
    // endpoint framework. The response alias keeps Platform's owned encoder
    // signature uniform, including the unit fault acknowledgement.
    (@method $name:ident, $kind:ident, $private:literal, $($id:ident)::+, $limits:ident,
        [$($asynchronous:ident)?] ($($input:ident: $request:ty)?) -> $reply:ty,
        $body:expr, $decode_error:literal, $encode_error:literal) => {
        mod $name {
            use $crate::endpoint::{cdk, protocol, wire};
            type Reply = $reply;

            fn admit() -> Result<(), String> {
                $crate::endpoint::admit(protocol::$limits.bytes, $private)
            }
            #[expect(clippy::needless_pass_by_value, reason = "Platform transfers the argument buffer to its decoder")]
            fn decode(bytes: Vec<u8>) -> ($($request)?) {
                wire::native::decode_request(protocol::$($id)::+, &bytes, protocol::$limits)
                    .expect($decode_error)
            }
            #[allow(clippy::needless_pass_by_value, reason = "Platform passes an owned reply; selected reply types include both Copy and non-Copy values")]
            fn encode(value: Reply) -> Vec<u8> {
                wire::native::encode_response(
                    protocol::$($id)::+, &value,
                    $crate::endpoint::response_limits(protocol::$($id)::+),
                )
                    .expect($encode_error)
            }
            #[cdk::$kind(crate = "cdk", guard = "admit", decode_with = "decode", encode_with = "encode")]
            $($asynchronous)? fn $name($($input: $request)?) -> Reply {
                $body
            }
        }
    };
}
