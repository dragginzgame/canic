//! Module: ops::ic::build_network
//!
//! Responsibility: expose build-network runtime metadata.
//! Does not own: build-network configuration, deployment selection, or CLI flags.
//! Boundary: ops facade over infra build-network discovery.

use crate::infra::ic::build_network::BuildNetworkInfra;
use canic_contracts::ids::BuildNetwork;

///
/// BuildNetworkOps
///
/// Operations-layer facade for build-network metadata.
///

pub struct BuildNetworkOps;

impl BuildNetworkOps {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        BuildNetworkInfra::build_network()
    }
}
