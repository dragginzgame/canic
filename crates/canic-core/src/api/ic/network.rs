use crate::{ids::BuildNetwork, workflow::ic::network::NetworkWorkflow};

///
/// NetworkApi
///

pub struct NetworkApi;

impl NetworkApi {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        NetworkWorkflow::build_network()
    }
}
