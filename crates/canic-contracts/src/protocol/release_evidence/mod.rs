//! Controller-only, read-only discovery of original release obligations.
/// Cursor-only request budget.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 4096,
    nodes: 256,
    depth: 8,
};
/// Preserve the existing bounded one-row response ceiling.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 262_144,
    allocation: 2_097_152,
    nodes: 262_144,
    depth: 64,
};
/// Canonical accounting obligations, including unlinked reservations.
pub mod intents {
    pub const METHOD: &str = "canic_release_intents";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0a00;
}
/// Retained replay receipts and their original uncertain effects.
pub mod receipts {
    pub const METHOD: &str = "canic_release_receipts";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0a01;
}

#[cfg(test)]
mod tests;
