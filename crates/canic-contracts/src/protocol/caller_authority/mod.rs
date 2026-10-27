//! Root-owned publication transitions; hashes and replay remain owned by the existing engine.
pub const PREPARE: &str = "canic_caller_authority_prepare";
pub const PREPARE_ID: u64 = 0x4341_4e49_0000_0400;
pub const COMMIT: &str = "canic_caller_authority_commit";
pub const COMMIT_ID: u64 = 0x4341_4e49_0000_0401;
pub const COMPLETE: &str = "canic_caller_authority_complete";
pub const COMPLETE_ID: u64 = 0x4341_4e49_0000_0402;
pub const RELEASE: &str = "canic_application_startup_release";
pub const RELEASE_ID: u64 = 0x4341_4e49_0000_0403;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 64,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 64,
};
