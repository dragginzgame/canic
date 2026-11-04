//! Current controller-authorized application drain contract, explicitly selected in Fleet desired state.

pub const METHOD: &str = "canic_cycle_drain";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_3b00;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};

#[cfg(test)]
mod tests;
