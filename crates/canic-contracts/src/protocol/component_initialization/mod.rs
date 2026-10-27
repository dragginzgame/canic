//! Root application initialization binding retains the prior maximum ingress budget.
pub const METHOD: &str = "canic_root_bind_component_initialization";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1200;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::dto::component_registry::MAX_COMPONENT_APPLICATION_INIT_BYTES
        + crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
    allocation: 262_144,
    nodes: 65_536,
    depth: 64,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 262_144,
    nodes: 16_384,
    depth: 64,
};

#[cfg(test)]
mod tests;
