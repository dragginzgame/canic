pub use crate::infra::ic::network::Network;

#[must_use]
pub fn network() -> Option<Network> {
    crate::infra::ic::build_network()
}
