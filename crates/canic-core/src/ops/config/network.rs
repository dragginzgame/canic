use crate::infra::ic as infra_ic;

pub use infra_ic::Network;

#[must_use]
pub fn build_network() -> Option<Network> {
    infra_ic::build_network()
}

#[must_use]
pub fn build_network_from_dfx_network(dfx_network: Option<&'static str>) -> Option<Network> {
    infra_ic::build_network_from_dfx_network(dfx_network)
}
