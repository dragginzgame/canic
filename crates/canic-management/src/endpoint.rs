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
    if method == protocol::custody::ROOT {
        return protocol::custody::ROOT_LIMITS;
    }
    if matches!(
        method,
        protocol::store_maintenance::STAGE
            | protocol::store_maintenance::ADVANCE
            | protocol::store_maintenance::OBSERVE
            | protocol::store_maintenance::INSTALL
            | protocol::store_maintenance::ACTIVATE
            | protocol::store_maintenance::STATUS
            | protocol::store_maintenance::RECOVER
    ) {
        return protocol::store_maintenance::LIMITS;
    }
    #[cfg(feature = "fault-injection")]
    if crate::engine::reply_fault(method) {
        return wire::Limits {
            bytes: 0,
            ..protocol::LIMITS
        };
    }
    if matches!(
        method,
        protocol::publication::EXECUTE
            | protocol::publication::INFO
            | protocol::publication::CUSTODY
            | protocol::publication::ADMISSION
    ) {
        return protocol::publication::RESPONSE_LIMITS;
    }
    if method == protocol::stored::CUSTODY {
        return protocol::stored::LIMITS;
    }
    if method == protocol::children::CUSTODY {
        return protocol::children::LIMITS;
    }
    if method == protocol::custody::OBSERVE {
        return protocol::custody::LIMITS;
    }
    #[cfg(not(feature = "fault-injection"))]
    let _ = method;
    protocol::LIMITS
}

/// Mount explicitly selected management methods using Platform's IC entrypoints.
///
/// Names are the existing exports: `status`, `observe`, `observe_custody`, `stage`, `reconcile`,
/// `stage_upgrade`, `upgrade_status`, `observe_upgrade`, `reconcile_upgrade`,
/// and (only with the supplier's `fault-injection` feature) `inject_fault`.
/// Child-scoped selections: `child_status`, `child_upgrade_status`, `observe_child`, `observe_children`,
/// `stage_child`, `reconcile_child`, `stage_child_upgrade`, `observe_child_upgrade`,
/// `reconcile_child_upgrade` and test-only `inject_child_fault`.
/// Store-bound selections: `observe_stored_children`, `stage_stored_child`,
/// `reconcile_stored_child`, `stage_stored_upgrade` and `reconcile_stored_upgrade`.
/// `store_publication` mounts a separate, controller-only bounded publication call.
/// `observe_store` reads initialized Store state through Root without claiming deployment.
/// `observe_store_custody` reads one release and all pins on its shared artifact.
/// `store_admission` forwards an exact Store close/observe/activate command, not an upgrade.
/// `stage_store_maintenance`, `advance_store_maintenance` and `observe_store_maintenance`
/// retain and recover Root-owned closure/stopping.
/// `install_store_maintenance` supplies approved replacement bytes; `activate_store_maintenance`
/// explicitly opens admission only after replacement verification.
/// `store_maintenance_status` reports the same retained metadata as an uncertified public query.
/// `recover_store_maintenance` retains observed outcomes without issuing fresh maintenance effects.
/// `observe_root_custody` reads children, artifact references and Store maintenance together.
/// Duplicate selections fail compilation. Unknown names have no expansion.
/// This mounts no lifecycle hooks, storage, timers or readiness policy.
///
/// ```ignore
/// canic_management::mount_endpoints! {
///     status, observe, observe_custody, stage, reconcile,
///     stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
/// }
/// ```
#[macro_export]
macro_rules! mount_endpoints {
    (@select observe_root_custody) => {
        $crate::mount_endpoints!(@method observe_root_custody, update, true, custody::ROOT, LIMITS,
            [] () -> $crate::endpoint::protocol::custody::Root,
            $crate::stored::root_custody(), "Root custody request", "Root custody reply");
    };
    ($($name:ident),+ $(,)?) => { $($crate::mount_endpoints!(@select $name);)+ };
    (@select install_store_maintenance) => {
        $crate::mount_endpoints!(@method install_store_maintenance, update, true, store_maintenance::INSTALL, STORE_INSTALL_LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Install) -> Result<$crate::endpoint::protocol::store_maintenance::Status, $crate::endpoint::protocol::Refusal>,
            $crate::store_maintenance::install(input).await, "Store installation request", "Store installation reply");
    };
    (@select activate_store_maintenance) => {
        $crate::mount_endpoints!(@method activate_store_maintenance, update, true, store_maintenance::ACTIVATE, LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> $crate::endpoint::protocol::store_maintenance::Status,
            $crate::store_maintenance::activate(input).await, "Store activation request", "Store activation reply");
    };
    (@select stage_store_maintenance) => {
        $crate::mount_endpoints!(@method stage_store_maintenance, update, true, store_maintenance::STAGE, LIMITS,
            [] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> Result<$crate::endpoint::protocol::store_maintenance::Status, $crate::endpoint::protocol::Refusal>,
            $crate::store_maintenance::stage(input), "Store maintenance request", "Store maintenance reply");
    };
    (@select advance_store_maintenance) => {
        $crate::mount_endpoints!(@method advance_store_maintenance, update, true, store_maintenance::ADVANCE, LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> $crate::endpoint::protocol::store_maintenance::Status,
            $crate::store_maintenance::advance(input).await, "Store maintenance request", "Store maintenance reply");
    };
    (@select observe_store_maintenance) => {
        $crate::mount_endpoints!(@method observe_store_maintenance, update, true, store_maintenance::OBSERVE, LIMITS,
            [] () -> Option<$crate::endpoint::protocol::store_maintenance::Status>,
            $crate::store_maintenance::observe(), "Store maintenance request", "Store maintenance reply");
    };
    (@select recover_store_maintenance) => {
        $crate::mount_endpoints!(@method recover_store_maintenance, update, true, store_maintenance::RECOVER, LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> $crate::endpoint::protocol::store_maintenance::Status,
            $crate::store_maintenance::recover(input).await, "Store recovery request", "Store recovery reply");
    };
    (@select store_maintenance_status) => {
        $crate::mount_endpoints!(@method store_maintenance_status, query, false, store_maintenance::STATUS, LIMITS,
            [] () -> Option<$crate::endpoint::protocol::store_maintenance::Status>,
            $crate::store_maintenance::observe(), "Store maintenance request", "Store maintenance reply");
    };
    (@select store_admission) => {
        $crate::mount_endpoints!(@method store_admission, update, true, publication::ADMISSION, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::AdmissionRequest) -> $crate::endpoint::protocol::publication::AdmissionReply,
            $crate::publication::admission(input).await, "Store admission request", "Store admission reply");
    };
    (@select observe_store_custody) => {
        $crate::mount_endpoints!(@method observe_store_custody, update, true, publication::CUSTODY, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::CustodyRequest) -> $crate::endpoint::protocol::publication::CustodyReply,
            $crate::publication::observe_custody(input).await, "Store custody request", "Store custody reply");
    };
    (@select observe_store) => {
        $crate::mount_endpoints!(@method observe_store, update, true, publication::INFO, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::InfoRequest) -> $crate::endpoint::protocol::publication::InfoReply,
            $crate::publication::observe_info(input).await, "Store observation request", "Store observation reply");
    };
    (@select store_publication) => {
        $crate::mount_endpoints!(@method store_publication, update, true, publication::EXECUTE, PUBLICATION_LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::Request) -> $crate::endpoint::protocol::publication::Reply,
            $crate::publication::execute(input).await, "publication request", "publication reply");
    };
    (@select observe_stored_children) => {
        $crate::mount_endpoints!(@method observe_stored_children, update, true, stored::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::stored::Custody,
            $crate::stored::custody(), "Store custody request", "Store custody reply");
    };
    (@select stage_stored_child) => {
        $crate::mount_endpoints!(@method stage_stored_child, update, true, stored::STAGE, STORED_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::Approval>>) -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stored::stage(input.slot, input.value), "Store request", "Store reply");
    };
    (@select reconcile_stored_child) => {
        $crate::mount_endpoints!(@method reconcile_stored_child, update, true, stored::RECONCILE, STORED_LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::Approval>>) -> $crate::endpoint::protocol::Status,
            $crate::stored::reconcile(input.slot, input.value).await, "Store request", "Store reply");
    };
    (@select stage_stored_upgrade) => {
        $crate::mount_endpoints!(@method stage_stored_upgrade, update, true, stored::STAGE_UPGRADE, STORED_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::upgrade::Approval>>) -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stored::stage_upgrade(input.slot, input.value), "Store request", "Store reply");
    };
    (@select reconcile_stored_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_stored_upgrade, update, true, stored::RECONCILE_UPGRADE, STORED_LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::upgrade::Approval>>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::stored::reconcile_upgrade(input.slot, input.value).await, "Store request", "Store reply");
    };
    (@select observe_children) => {
        $crate::mount_endpoints!(@method observe_children, update, true, children::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::children::Custody,
            $crate::children::custody(), "child request", "child reply");
    };
    (@select child_status) => {
        $crate::mount_endpoints!(@method child_status, query, false, children::STATUS, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Slot) -> $crate::endpoint::protocol::Status,
            $crate::children::status(input), "child request", "child reply");
    };
    (@select observe_child) => {
        $crate::mount_endpoints!(@method observe_child, update, true, children::STATUS, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Slot) -> $crate::endpoint::protocol::Status,
            $crate::children::status(input), "child request", "child reply");
    };
    (@select stage_child) => {
        $crate::mount_endpoints!(@method stage_child, update, true, children::STAGE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::Stage>) -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::children::stage(input.slot, input.value), "child request", "child reply");
    };
    (@select reconcile_child) => {
        $crate::mount_endpoints!(@method reconcile_child, update, true, children::RECONCILE, LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::Approval>) -> $crate::endpoint::protocol::Status,
            $crate::children::reconcile(input.slot, input.value).await, "child request", "child reply");
    };
    (@select stage_child_upgrade) => {
        $crate::mount_endpoints!(@method stage_child_upgrade, update, true, children::STAGE_UPGRADE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Stage>) -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::children::stage_upgrade(input.slot, input.value), "child request", "child reply");
    };
    (@select observe_child_upgrade) => {
        $crate::mount_endpoints!(@method observe_child_upgrade, update, true, children::OBSERVE_UPGRADE, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Approval>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::children::observe_upgrade(input.slot, &input.value), "child request", "child reply");
    };
    (@select child_upgrade_status) => {
        $crate::mount_endpoints!(@method child_upgrade_status, query, false, children::OBSERVE_UPGRADE, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Approval>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::children::observe_upgrade(input.slot, &input.value), "child request", "child reply");
    };
    (@select reconcile_child_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_child_upgrade, update, true, children::RECONCILE_UPGRADE, LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Approval>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::children::reconcile_upgrade(input.slot, input.value).await, "child request", "child reply");
    };
    (@select inject_child_fault) => {
        $crate::mount_endpoints!(@method inject_child_fault, update, true, children::INJECT, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<u8>) -> (),
            $crate::children::inject_fault(input.slot, input.value), "child request", "child reply");
    };
    (@select observe_custody) => {
        $crate::mount_endpoints!(@method observe_custody, update, true, custody::OBSERVE, LIMITS,
            [] () -> $crate::endpoint::protocol::custody::Custody,
            $crate::custody(), "custody frame", "custody reply");
    };
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
        $crate::mount_endpoints!(@method observe_upgrade, update, true, upgrade::OBSERVE, LIMITS,
            [] (expected: $crate::endpoint::protocol::upgrade::Approval) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::observe_upgrade(&expected), "upgrade status frame", "upgrade status reply");
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
                wire::decode(&bytes, wire::Encoding::Frame { method: protocol::$($id)::+, direction: wire::Direction::Request }, protocol::$limits)
                    .expect($decode_error)
            }
            #[allow(clippy::needless_pass_by_value, reason = "Platform passes an owned reply; selected reply types include both Copy and non-Copy values")]
            fn encode(value: Reply) -> Vec<u8> {
                wire::encode(&value, wire::Encoding::Frame { method: protocol::$($id)::+, direction: wire::Direction::Response }, $crate::endpoint::response_limits(protocol::$($id)::+))
                    .expect($encode_error)
            }
            #[cdk::$kind(crate = "cdk", guard = "admit", decode_with = "decode", encode_with = "encode")]
            $($asynchronous)? fn $name($($input: $request)?) -> Reply {
                $body
            }
        }
    };
}
