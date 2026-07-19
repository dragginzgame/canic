//! Module: infra::ic::build_network
//!
//! Responsibility: expose the build-time IC network class.
//! Does not own: build-network configuration, config validation, or endpoint policy.
//! Boundary: ops and access predicates call this for baked-in build-network state.

use crate::ids::BuildNetwork;

///
/// BuildNetworkInfra
///
/// Build-time IC network facade.
/// Owned by IC infra and used where compiled build-network identity is required.
///

pub struct BuildNetworkInfra;

impl BuildNetworkInfra {
    /// Return the build network inferred from `ICP_ENVIRONMENT`.
    ///
    /// This value is baked into the Wasm and does not reflect runtime state.
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        Self::build_network_from_icp_environment(option_env!("ICP_ENVIRONMENT"))
    }

    /// Parse the build-time `ICP_ENVIRONMENT` value used by `build_network`.
    #[must_use]
    pub fn build_network_from_icp_environment(
        icp_environment: Option<&'static str>,
    ) -> Option<BuildNetwork> {
        match icp_environment {
            Some("local") | None => Some(BuildNetwork::Local),
            Some("ic") => Some(BuildNetwork::Ic),
            _ => None,
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_network_from_icp_environment_parses_ic() {
        assert_eq!(
            BuildNetworkInfra::build_network_from_icp_environment(Some("ic")),
            Some(BuildNetwork::Ic)
        );
    }

    #[test]
    fn build_network_from_icp_environment_parses_local() {
        assert_eq!(
            BuildNetworkInfra::build_network_from_icp_environment(Some("local")),
            Some(BuildNetwork::Local)
        );
    }

    #[test]
    fn build_network_from_icp_environment_rejects_unknown() {
        assert_eq!(
            BuildNetworkInfra::build_network_from_icp_environment(Some("nope")),
            None
        );
    }

    #[test]
    fn build_network_from_icp_environment_handles_missing() {
        assert_eq!(
            BuildNetworkInfra::build_network_from_icp_environment(None),
            Some(BuildNetwork::Local)
        );
    }
}
