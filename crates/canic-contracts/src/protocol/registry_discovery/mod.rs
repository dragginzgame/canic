//! Fixed controller-protected Coordinator registry identities.
/// Unit request envelope.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
/// Bounded native manifest/version and application errors.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 262_144,
    depth: 64,
};
/// Canonical snapshot manifest.
pub mod manifest {
    pub const METHOD: &str = "canic_coordinator_registry_manifest";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0900;
}
/// Current exact registry head.
pub mod version {
    pub const METHOD: &str = "canic_coordinator_registry_version";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0901;
}

#[cfg(test)]
mod tests;

/// Complete canonical registry snapshot with registry-caller admission.
pub mod snapshot {
    pub const METHOD: &str = "canic_coordinator_registry";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0902;
}

/// Protected Root physical summary and exact active registry head.
pub mod root_inventory {
    pub const METHOD: &str = "canic_root_inventory";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0903;
}
