//! Separate Root capability methods retain caller and operation authority in their envelopes.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
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
pub mod acknowledge {
    pub const METHOD: &str = "canic_root_capability_acknowledge";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1000;
}
pub mod allocate {
    pub const METHOD: &str = "canic_root_capability_allocate";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1001;
}
pub mod create {
    pub const METHOD: &str = "canic_root_capability_create";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1002;
}
pub mod recycle {
    pub const METHOD: &str = "canic_root_capability_recycle";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1003;
}
pub mod cycles {
    pub const METHOD: &str = "canic_root_capability_cycles";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1004;
}
