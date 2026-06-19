//! Module: ops::ic::network
//!
//! Responsibility: expose build-network runtime metadata.
//! Does not own: network configuration, deployment selection, or CLI flags.
//! Boundary: ops facade over infra build-network discovery.

use crate::{ids::BuildNetwork, infra::ic::network::NetworkInfra};

///
/// NetworkOps
///
/// Operations-layer facade for build-network metadata.
///

pub struct NetworkOps;

impl NetworkOps {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        NetworkInfra::build_network()
    }
}
