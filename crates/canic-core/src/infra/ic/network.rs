use crate::ids::BuildNetwork;

///
/// NetworkInfra
///

pub struct NetworkInfra;

impl NetworkInfra {
    ///
    /// build_network
    /// Returns the network inferred at *build time* from `DFX_NETWORK`.
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
        Self::build_network_from_dfx_network(option_env!("DFX_NETWORK"))
    }

    ///
    /// build_network_from_dfx_network
    /// Pure helper for `build_network()`
    ///

    #[must_use]
    pub fn build_network_from_dfx_network(
        dfx_network: Option<&'static str>,
    ) -> Option<BuildNetwork> {
        match dfx_network {
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
    fn build_network_from_dfx_network_parses_ic() {
        assert_eq!(
            NetworkInfra::build_network_from_dfx_network(Some("ic")),
            Some(BuildNetwork::Ic)
        );
    }

    #[test]
    fn build_network_from_dfx_network_parses_local() {
        assert_eq!(
            NetworkInfra::build_network_from_dfx_network(Some("local")),
            Some(BuildNetwork::Local)
        );
    }

    #[test]
    fn build_network_from_dfx_network_rejects_unknown() {
        assert_eq!(
            NetworkInfra::build_network_from_dfx_network(Some("nope")),
            None
        );
    }

    #[test]
    fn build_network_from_dfx_network_handles_missing() {
        assert_eq!(
            NetworkInfra::build_network_from_dfx_network(None),
            Some(BuildNetwork::Local)
        );
    }
}
