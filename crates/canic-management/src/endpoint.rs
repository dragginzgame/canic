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
    if crate::sharding::occupied() && !placement_admits(private, _method) {
        return Err("shard pool custody required".into());
    }
    if matches!(
        _method,
        protocol::application::CUSTODY_PAGE | protocol::store_maintenance::MAINTAIN
    ) && crate::backup::occupied()
    {
        return Err("active child snapshot prevents fleet maintenance".into());
    }
    if ic_platform_runtime::system::argument_size() > limit {
        return Err("management frame too large".into());
    }
    Ok(())
}

#[cfg(feature = "sharding")]
fn placement_admits(private: bool, method: u64) -> bool {
    use protocol::{application, children, publication, store_maintenance};
    match method {
        children::READ_STATUS | store_maintenance::STATUS => true,
        children::INJECT => private && cfg!(feature = "fault-injection"),
        application::STAGE_CHILD_CREATION => private && crate::sharding::has_capacity(),
        children::OBSERVE_STATUS
        | store_maintenance::READ_ATTEMPT
        | publication::EXECUTE
        | publication::OBSERVATION
        | application::CHILD_EVIDENCE
        | application::STAGE_CHILD_UPGRADE
        | application::RECONCILE_CHILD_UPGRADE
        | application::RECONCILE_CHILD_CREATION => private,
        _ => false,
    }
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
    declared_response_limits(method)
}

/// Wire budget advertised by the mounted contract.
#[doc(hidden)]
#[must_use]
pub const fn declared_response_limits(method: u64) -> wire::Limits {
    if matches!(
        method,
        protocol::application::CUSTODY
            | protocol::application::CUSTODY_PAGE
            | protocol::application::CHILD_EVIDENCE
            | protocol::application::ACTIVATE
    ) {
        return protocol::application::LIMITS;
    }
    if matches!(
        method,
        protocol::store_maintenance::MAINTAIN
            | protocol::store_maintenance::READ_ATTEMPT
            | protocol::store_maintenance::STATUS
            | protocol::store_maintenance::RECOVER
    ) {
        return protocol::store_maintenance::LIMITS;
    }
    if matches!(
        method,
        protocol::publication::EXECUTE
            | protocol::publication::OBSERVATION
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
    protocol::LIMITS
}

/// Trap a refused endpoint stage.
/// # Panics
/// Always traps, preserving synchronous rollback.
pub fn fail(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("management endpoint: {error:?}"))
}
