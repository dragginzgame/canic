//! Root renewal reporting has its own method identity and bounded native graph.
pub const METHOD: &str = "canic_root_issuer_renewal";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1100;
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

#[cfg(test)]
mod tests;
