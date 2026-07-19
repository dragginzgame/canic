//! Module: workflow::ic::build_network
//!
//! Responsibility: expose build-network detection to workflow callers.
//! Does not own: build-network configuration, IC calls, or endpoint DTOs.
//! Boundary: delegates build-network discovery to IC ops.

use crate::{ids::BuildNetwork, ops::ic::build_network::BuildNetworkOps};

///
/// BuildNetworkWorkflow
///
/// Workflow facade for build-network metadata.
///

pub struct BuildNetworkWorkflow;

impl BuildNetworkWorkflow {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        BuildNetworkOps::build_network()
    }
}
