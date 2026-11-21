//! Explicit management endpoint mounting. Platform owns execution; Wire owns codecs.

#[doc(hidden)]
pub use canic::__canic_inspect as inspect;
#[doc(hidden)]
pub use canic_management_protocol as protocol;
#[doc(hidden)]
pub use ic_platform_runtime as runtime;
#[doc(hidden)]
pub use ic_wire as wire;

/// Admission before Platform copies arguments. Authorization deliberately comes first.
///
/// # Errors
/// Refuses non-controllers on private methods, then oversized request frames.
#[doc(hidden)]
pub fn admit(limit: usize, private: bool, _method: u64) -> Result<(), String> {
    if private && !runtime::system::is_controller(&runtime::system::caller()) {
        return Err("controller required".into());
    }
    #[cfg(feature = "sharding")]
    if crate::sharding::occupied()
        && !(cfg!(feature = "fault-injection") && private && _method == protocol::children::INJECT)
        && !(private
            && crate::sharding::has_capacity()
            && matches!(
                _method,
                protocol::children::STAGE
                    | protocol::application::STAGE
                    | protocol::stored::STAGE
                    | protocol::children::RECONCILE
            ))
        && !(private
            && matches!(
                _method,
                protocol::stored::RECONCILE
                    | protocol::application::RECONCILE
                    | protocol::children::STATUS
            ))
    {
        return Err("shard pool custody required".into());
    }
    #[cfg(feature = "backup")]
    if private && crate::backup::occupied() {
        return Err("snapshot custody fences ordinary management".into());
    }
    if ic_platform_runtime::system::argument_size() > limit {
        return Err("growth frame too large".into());
    }
    Ok(())
}

/// Response policy, with a deliberately impossible frame budget in fault builds.
#[doc(hidden)]
#[must_use]
pub fn response_limits(method: u64) -> wire::Limits {
    if matches!(
        method,
        protocol::application::CUSTODY
            | protocol::application::CHILD_EVIDENCE
            | protocol::application::ACTIVATE
    ) {
        return protocol::application::LIMITS;
    }
    if method == protocol::custody::ROOT_HISTORY {
        return protocol::custody::ROOT_HISTORY_LIMITS;
    }
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
            | protocol::store_maintenance::OBSERVE_ATTEMPT
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
            | protocol::publication::REMOVE
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

/// Trap a refused endpoint stage.
/// # Panics
/// Always traps, preserving synchronous rollback.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("management endpoint: {error:?}"))
}
