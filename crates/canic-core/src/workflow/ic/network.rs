use crate::{ids::BuildNetwork, ops::ic::network::NetworkOps};

///
/// NetworkWorkflow
///

pub struct NetworkWorkflow;

impl NetworkWorkflow {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        NetworkOps::build_network()
    }
}
