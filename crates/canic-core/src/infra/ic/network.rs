use std::fmt::{self, Display};

///
/// BuildNetwork
/// Identifies the environment the canister believes it runs in.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildNetwork {
    Ic,
    Local,
}

impl BuildNetwork {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ic => "ic",
            Self::Local => "local",
        }
    }
}

impl Display for BuildNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

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
    build_network_from_dfx_network(option_env!("DFX_NETWORK"))
}

///
/// build_network_from_dfx_network
/// Pure helper for `build_network()`, exposed for testing and reuse.
///

#[must_use]
pub fn build_network_from_dfx_network(dfx_network: Option<&'static str>) -> Option<BuildNetwork> {
    match dfx_network {
        Some("local") => Some(BuildNetwork::Local),
        Some("ic") => Some(BuildNetwork::Ic),

        _ => None,
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
            build_network_from_dfx_network(Some("ic")),
            Some(BuildNetwork::Ic)
        );
    }

    #[test]
    fn build_network_from_dfx_network_parses_local() {
        assert_eq!(
            build_network_from_dfx_network(Some("local")),
            Some(BuildNetwork::Local)
        );
    }

    #[test]
    fn build_network_from_dfx_network_rejects_unknown() {
        assert_eq!(build_network_from_dfx_network(Some("nope")), None);
    }

    #[test]
    fn build_network_from_dfx_network_handles_missing() {
        assert_eq!(build_network_from_dfx_network(None), None);
    }
}
