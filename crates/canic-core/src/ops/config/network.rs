use crate::infra::ic as infra_ic;

pub use infra_ic::Network;

#[must_use]
pub fn build_network() -> Option<Network> {
    infra_ic::build_network()
}
