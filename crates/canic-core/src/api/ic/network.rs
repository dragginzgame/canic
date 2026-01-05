pub use crate::infra::ic::network::Network;

///
/// NetworkApi
///

pub struct NetworkApi;

impl NetworkApi {
    #[must_use]
    pub fn network() -> Option<Network> {
        crate::infra::ic::build_network()
    }
}
