use crate::infra::{self, network::Network};

///
/// NetworkOps
///
/// Ops-level access to the current execution network.
///
/// This module exposes a stable façade for querying the runtime network
/// (e.g. Local vs IC) without leaking infra-level detection details into
/// workflow or policy layers.
///

pub struct NetworkOps;

impl NetworkOps {
    #[must_use]
    pub fn current_network() -> Option<Network> {
        infra::ic::build_network()
    }
}
