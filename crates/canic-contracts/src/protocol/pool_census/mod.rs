//! Bounded retained pool obligations; observation never grants settlement authority.
pub const METHOD: &str = "canic_root_pool_release";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1e00;
pub const REQUEST_LIMITS: ic_wire::Limits = crate::protocol::store_inventory::REQUEST_LIMITS;
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1_048_576,
    allocation: 8_388_608,
    nodes: 1_048_576,
    depth: 64,
};

#[cfg(all(test, feature = "wire-schema"))]
mod tests;
