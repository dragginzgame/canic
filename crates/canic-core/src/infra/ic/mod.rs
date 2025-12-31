//! IC-related infra helpers.
//!
//! This module groups low-level IC concerns (management canister calls, ICC call
//! wrappers, HTTP outcalls, timers) under a single namespace to keep `infra/`
//! navigable.

pub mod call;
pub mod cmc;
pub mod http;
pub mod ledger;
pub mod mgmt;
pub mod nns;
pub mod signature;

use crate::infra::prelude::*;

///
/// IcInfraError
///

#[derive(Debug, ThisError)]
pub enum IcInfraError {
    #[error(transparent)]
    HttpInfra(#[from] http::HttpInfraError),

    #[error(transparent)]
    LedgerInfra(#[from] ledger::LedgerInfraError),

    #[error(transparent)]
    MgmtInfra(#[from] mgmt::MgmtInfraError),

    #[error(transparent)]
    NnsInfra(#[from] nns::NnsInfraError),

    #[error(transparent)]
    SignatureInfra(#[from] signature::SignatureOpsError),
}

///
/// Network
/// Identifies the environment the canister believes it runs in.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Network {
    Ic,
    Local,
}

impl Network {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ic => "ic",
            Self::Local => "local",
        }
    }
}

impl core::fmt::Display for Network {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// build_network_from_dfx_network
/// Pure helper for `build_network()`, exposed for testing and reuse.
///

#[must_use]
pub fn build_network_from_dfx_network(dfx_network: Option<&'static str>) -> Option<Network> {
    match dfx_network {
        Some("local") => Some(Network::Local),
        Some("ic") => Some(Network::Ic),

        _ => None,
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
pub fn build_network() -> Option<Network> {
    build_network_from_dfx_network(option_env!("DFX_NETWORK"))
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
            Some(Network::Ic)
        );
    }

    #[test]
    fn build_network_from_dfx_network_parses_local() {
        assert_eq!(
            build_network_from_dfx_network(Some("local")),
            Some(Network::Local)
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
