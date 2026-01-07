pub use crate::ops::ic::network::BuildNetwork;

use crate::ops::ic::network::NetworkOps;

///
/// NetworkApi
///

pub struct NetworkApi;

impl NetworkApi {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        NetworkOps::build_network()
    }
}
