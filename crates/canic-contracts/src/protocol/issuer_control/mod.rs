//! Independent method identities for Root issuer administration.
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
pub mod configure {
    pub const METHOD: &str = "canic_root_configure_issuer";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1103;
}
pub mod public_key {
    pub const METHOD: &str = "canic_root_chain_key_public_key";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1104;
}
pub mod proof {
    pub const METHOD: &str = "canic_root_delegation_proof";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1105;
}
