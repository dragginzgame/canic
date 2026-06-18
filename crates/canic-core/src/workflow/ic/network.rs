//! Module: workflow::ic::network
//!
//! Responsibility: expose build-network detection to workflow callers.
//! Does not own: network configuration, IC calls, or endpoint DTOs.
//! Boundary: delegates network discovery to IC ops.

use crate::{ids::BuildNetwork, ops::ic::network::NetworkOps};

///
/// NetworkWorkflow
///
/// Workflow facade for build-network metadata.
///

pub struct NetworkWorkflow;

impl NetworkWorkflow {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        NetworkOps::build_network()
    }
}
