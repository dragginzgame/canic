use crate::ids::BuildNetwork;

///
/// NetworkInfra
///

pub struct NetworkInfra;

impl NetworkInfra {
    ///
    /// build_network
    /// Returns the network inferred at *build time* from `ICP_ENVIRONMENT`.
    /// This value is baked into the Wasm and does not reflect runtime state.
    ///
    /// ChatGPT 5.2 Final, Precise Verdict
    ///
    /// ✅ Yes, this works exactly as you say
    /// ✅ It is valid IC/Wasm code
    /// ❌ It is not runtime detection
    /// ⚠️ The danger is semantic, not technical
    /// ✅ Safe if treated as a build-time constant
    /// ❌ Dangerous if treated as authoritative runtime truth
    ///

    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        Self::build_network_from_icp_environment(option_env!("ICP_ENVIRONMENT"))
    }

    ///
    /// build_network_from_icp_environment
    /// Pure helper for `build_network()`
    ///

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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_network_from_icp_environment_parses_ic() {
        assert_eq!(
            NetworkInfra::build_network_from_icp_environment(Some("ic")),
            Some(BuildNetwork::Ic)
        );
    }

    #[test]
    fn build_network_from_icp_environment_parses_local() {
        assert_eq!(
            NetworkInfra::build_network_from_icp_environment(Some("local")),
            Some(BuildNetwork::Local)
        );
    }

    #[test]
    fn build_network_from_icp_environment_rejects_unknown() {
        assert_eq!(
            NetworkInfra::build_network_from_icp_environment(Some("nope")),
            None
        );
    }

    #[test]
    fn build_network_from_icp_environment_handles_missing() {
        assert_eq!(
            NetworkInfra::build_network_from_icp_environment(None),
            Some(BuildNetwork::Local)
        );
    }
}
