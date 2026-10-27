//! Operation-specific Store publication controls retain existing admission budgets.
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
pub mod inspect {
    pub const METHOD: &str = "canic_wasm_store_inspect_template";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1400;
}
pub mod prepare {
    pub const METHOD: &str = "canic_wasm_store_prepare_chunk_set";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1401;
}
pub mod stage {
    pub const METHOD: &str = "canic_wasm_store_stage_manifest";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1402;
}

pub mod fixture_prepare {
    pub const METHOD: &str = "canic_wasm_store_prepare_fixture";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1403;
}

#[cfg(test)]
mod tests;
