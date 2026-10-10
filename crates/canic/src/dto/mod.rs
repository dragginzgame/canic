//! Supported facade for the canonical shared contract vocabulary.

pub use canic_contracts::dto::*;

/// Coordinator commands and its runtime-coupled installation input.
#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator {
    pub use canic_contracts::dto::fleet_coordinator::*;
    pub use canic_control_plane::installation::FleetCoordinatorInitArgs;
}
