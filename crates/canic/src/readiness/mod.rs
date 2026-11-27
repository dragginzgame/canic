//! Bounded application observations, independent of lifecycle and recovery authority.

#[cfg(test)]
mod tests;

#[cfg(feature = "endpoints")]
pub mod endpoint;

use serde::{Deserialize, Serialize};

/// Identity of the replicated, observational `readiness` method.
pub const METHOD: u64 = 3;
/// Empty request: no caller-supplied authority, state or activation attempt.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 32,
    allocation: 256,
    nodes: 4,
    depth: 4,
};
/// One fixed application state; no arbitrary diagnostic payload.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 32,
    allocation: 256,
    nodes: 4,
    depth: 4,
};

/// Application-owned state at observation time, not permission or a readiness lease.
/// Observers must bind the response to their approved target and executable.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum State {
    /// The application has not established its prerequisites for service.
    /// This includes unavailable or unopened state; observation must not initialize it.
    Unavailable,
    /// State is open, but ordinary writes remain closed by application policy.
    /// Observation cannot acknowledge, replace or clear a maintenance/recovery fence.
    Fenced,
    /// The application currently admits ordinary writes under its own rules.
    /// This grants no user permission and does not prove persistence or future availability.
    Ready,
}

/// Mount a public replicated observation using an explicit synchronous application handler.
/// The handler returns `readiness::State` without changing application state.
/// Mounting initializes no storage, timers or services and grants no activation authority.
#[cfg(feature = "endpoints")]
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_readiness_endpoint {
    ($snapshot:path) => {
        const _: () = {
            use $crate::__private::runtime;
            use $crate::readiness::{State, endpoint};

            $crate::__canic_builtin! {
                wire, $crate::readiness::METHOD, ($crate::readiness::REQUEST_LIMITS, $crate::readiness::RESPONSE_LIMITS), runtime::update, "canic_readiness", (
                    runtime = runtime,
                    guard = endpoint::admit,
                    decode = endpoint::decode,
                    max_bytes = 32,
                    on_error = endpoint::fail,
                    encode = endpoint::encode,
                ),
                [] fn __canic_readiness((): ()) -> State {
                    $snapshot()
                }
            }
        };
    };
}
