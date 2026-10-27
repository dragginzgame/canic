//! Independent Store catalog and retained-storage observations.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 262_144,
    nodes: 16_384,
    depth: 64,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 1_048_576,
    depth: 64,
};
pub mod catalog {
    pub const METHOD: &str = "canic_wasm_store_inventory";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1300;
}
pub mod storage {
    pub const METHOD: &str = "canic_wasm_store_storage";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1301;
}

#[cfg(test)]
mod tests;
