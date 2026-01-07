pub use crate::infra::ic::network::BuildNetwork;

///
/// NetworkOps
///

pub struct NetworkOps;

impl NetworkOps {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        crate::infra::ic::network::build_network()
    }
}
