//! Controller-authorized installation of a complete Root-issued proof.
pub const INSTALL: &str = "canic_install_delegation_proof";
pub const INSTALL_ID: u64 = 0x4341_4e49_0000_0e00;
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
#[cfg(all(test, feature = "wire-schema"))]
mod tests {
    #[test]
    fn installation_schema_covers_the_complete_native_proof_graph() {
        ic_wire::typescript::native_schema::<
            crate::dto::issuer_proof::InstallActiveDelegationProofRequest,
        >()
        .unwrap();
        ic_wire::typescript::native_schema::<
            Result<
                crate::dto::issuer_proof::InstallActiveDelegationProofResponse,
                crate::dto::error::Error,
            >,
        >()
        .unwrap();
    }
}

/// Public read of issuer-local evidence, independently from installation.
pub mod status {
    pub const METHOD: &str = "canic_active_delegation_proof";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0e01;
    pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
        bytes: 128,
        allocation: 1024,
        nodes: 64,
        depth: 8,
    };
    pub const RESPONSE_LIMITS: ic_wire::Limits = super::RESPONSE_LIMITS;
}
