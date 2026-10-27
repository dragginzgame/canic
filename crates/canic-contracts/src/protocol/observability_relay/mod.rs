//! Replicated controller-only observation hops; fixed per-operation Wire identities.

/// Replicated child funding observation.
pub mod child_funding {
    /// Exact update method.
    pub const METHOD: &str = "canic_observe_child_funding";
    /// Native frame identity, distinct from the direct query.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0500;
    pub use crate::protocol::child_funding::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
/// Replicated cycle balance observation.
pub mod cycle_balance {
    /// Exact update method.
    pub const METHOD: &str = "canic_observe_cycle_balance";
    /// Native frame identity, distinct from the direct query.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0501;
    pub use crate::protocol::cycle_balance::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
/// Replicated cycle history observation.
pub mod cycle_history {
    /// Exact update method.
    pub const METHOD: &str = "canic_observe_cycle_history";
    /// Native frame identity, distinct from the direct query.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0502;
    pub use crate::protocol::cycle_history::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
/// Replicated cycle topups observation.
pub mod cycle_topups {
    /// Exact update method.
    pub const METHOD: &str = "canic_observe_cycle_topups";
    /// Native frame identity, distinct from the direct query.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0503;
    pub use crate::protocol::cycle_topups::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
/// Replicated memory allocations observation.
pub mod memory_allocations {
    /// Exact update method.
    pub const METHOD: &str = "canic_observe_memory_allocations";
    /// Native frame identity, distinct from the direct query.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0504;
    pub use crate::protocol::protected_memory_allocations::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
/// Replicated metrics observation.
pub mod metrics {
    /// Exact update method.
    pub const METHOD: &str = "canic_observe_metrics";
    /// Native frame identity, distinct from the direct query.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0505;
    pub use crate::protocol::protected_metrics::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
