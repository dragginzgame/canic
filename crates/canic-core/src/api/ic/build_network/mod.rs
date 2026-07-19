use crate::{ids::BuildNetwork, workflow::ic::build_network::BuildNetworkWorkflow};

///
/// BuildNetworkApi
///

pub struct BuildNetworkApi;

impl BuildNetworkApi {
    #[must_use]
    pub fn build_network() -> Option<BuildNetwork> {
        BuildNetworkWorkflow::build_network()
    }
}
