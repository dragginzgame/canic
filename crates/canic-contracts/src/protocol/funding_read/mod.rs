//! Controller-only retained funding evidence; this query grants no settlement authority.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 4096,
    nodes: 256,
    depth: 16,
};
/// Match the signed release collector's existing per-page byte ceiling.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256 * 1024,
    allocation: 2 * 1024 * 1024,
    nodes: 262_144,
    depth: 64,
};
pub mod release {
    pub const METHOD: &str = "canic_root_funding_release";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3100;
}

/// Live Root funding diagnostics, without funding mutation authority.
pub mod root_status {
    pub const METHOD: &str = "canic_root_funding";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3101;
}

/// Complete Coordinator treasury and Root funding evidence.
pub mod coordinator_status {
    pub const METHOD: &str = "canic_coordinator_funding";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3102;
}

#[cfg(test)]
mod tests;

/// Exact authorized retained operation observation; no mutation or lease extension.
pub mod rotation {
    pub const METHOD: &str = "canic_coordinator_funding_rotation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3301;
}
