pub use crate::infra::ic::network::{BuildNetwork, NetworkInfra};

///
/// NetworkOps
///

pub struct NetworkOps;

impl NetworkOps {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        NetworkInfra::build_network()
    }
}
