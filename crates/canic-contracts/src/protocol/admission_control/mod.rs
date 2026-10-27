//! Fixed admission methods, bounded independently from provisioning commands.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::protocol::limits::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
    allocation: 262_144,
    nodes: 16_384,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 262_144,
    depth: 64,
};
/// Protected active admission policy and current operation.
pub mod status {
    pub const METHOD: &str = "canic_coordinator_admission";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b00;
}
/// Exact controller-authorized policy mutation with existing operation replay semantics.
pub mod mutate {
    pub const METHOD: &str = "canic_coordinator_admission_mutate";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b01;
}

#[cfg(test)]
mod tests;

/// Protected Root participant page and original retained operation receipt.
pub mod root_status {
    pub const METHOD: &str = "canic_root_admission";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b02;
}

/// Managed projection query; its existing method name remains CANIC_ADMISSION_STATUS.
pub mod managed_status {
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b03;
}

/// Root-authorized prepare transition with retained operation receipts.
pub mod target_prepare {
    pub const METHOD: &str = "canic_admission_prepare";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b04;
}

/// Root-authorized activate transition with retained operation receipts.
pub mod target_activate {
    pub const METHOD: &str = "canic_admission_activate";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b05;
}

/// Root-authorized open transition with retained operation receipts.
pub mod target_open {
    pub const METHOD: &str = "canic_admission_open";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b06;
}

/// Exact Coordinator-authorized Root prepare intent.
pub mod root_prepare {
    pub const METHOD: &str = "canic_root_admission_prepare";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b07;
}

/// Exact Coordinator-authorized Root activate intent.
pub mod root_activate {
    pub const METHOD: &str = "canic_root_admission_activate";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b08;
}

/// Exact Coordinator-authorized Root open intent.
pub mod root_open {
    pub const METHOD: &str = "canic_root_admission_open";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0b09;
}

/// Exact authorized retained operation observation; no mutation or lease extension.
pub mod operation {
    pub const METHOD: &str = "canic_coordinator_admission_operation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3300;
}
