//! Root-authenticated retained managed runtime operation evidence.
pub const METHOD: &str = "canic_runtime_operation";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0d03;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 2048,
    nodes: 256,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 64,
};
#[cfg(test)]
mod tests;
