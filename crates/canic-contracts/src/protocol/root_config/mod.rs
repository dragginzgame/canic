//! Controller-only compiled configuration export; no runtime TOML parsing.

/// Exact method for the retained configuration text.
pub const METHOD: &str = "canic_root_config";
/// Stable identity of the bounded native Wire frames.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_010c;
/// Unit request envelope bounds.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
/// Text reply bounds within the IC query response limit.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 256,
    depth: 16,
};

#[cfg(test)]
mod tests;
