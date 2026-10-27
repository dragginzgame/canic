//! Fixed outer Root observation identities, distinct from the inner replicated hop.
//!
//! Controller-authenticated updates retain exact release-bound transport authority.

#[cfg(test)]
mod tests;

/// Outer requests include a maximum-length target and operation-specific input.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 4096,
    nodes: 128,
    depth: 16,
};

/// Root-routed child funding observation.
pub mod child_funding {
    /// Exact replicated update name.
    pub const METHOD: &str = "canic_root_observe_child_funding";
    /// Outer frame identity; never interchangeable with the inner hop.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0600;
    pub use super::REQUEST_LIMITS;
    pub use crate::protocol::observability_relay::child_funding::RESPONSE_LIMITS;
}

/// Root-routed cycle balance observation.
pub mod cycle_balance {
    /// Exact replicated update name.
    pub const METHOD: &str = "canic_root_observe_cycle_balance";
    /// Outer frame identity; never interchangeable with the inner hop.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0601;
    pub use super::REQUEST_LIMITS;
    pub use crate::protocol::observability_relay::cycle_balance::RESPONSE_LIMITS;
}

/// Root-routed cycle history observation.
pub mod cycle_history {
    /// Exact replicated update name.
    pub const METHOD: &str = "canic_root_observe_cycle_history";
    /// Outer frame identity; never interchangeable with the inner hop.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0602;
    pub use super::REQUEST_LIMITS;
    pub use crate::protocol::observability_relay::cycle_history::RESPONSE_LIMITS;
}

/// Root-routed cycle topups observation.
pub mod cycle_topups {
    /// Exact replicated update name.
    pub const METHOD: &str = "canic_root_observe_cycle_topups";
    /// Outer frame identity; never interchangeable with the inner hop.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0603;
    pub use super::REQUEST_LIMITS;
    pub use crate::protocol::observability_relay::cycle_topups::RESPONSE_LIMITS;
}

/// Root-routed memory allocations observation.
pub mod memory_allocations {
    /// Exact replicated update name.
    pub const METHOD: &str = "canic_root_observe_memory_allocations";
    /// Outer frame identity; never interchangeable with the inner hop.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0604;
    pub use super::REQUEST_LIMITS;
    pub use crate::protocol::observability_relay::memory_allocations::RESPONSE_LIMITS;
}

/// Root-routed metrics observation.
pub mod metrics {
    /// Exact replicated update name.
    pub const METHOD: &str = "canic_root_observe_metrics";
    /// Outer frame identity; never interchangeable with the inner hop.
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0605;
    pub use super::REQUEST_LIMITS;
    pub use crate::protocol::observability_relay::metrics::RESPONSE_LIMITS;
}
