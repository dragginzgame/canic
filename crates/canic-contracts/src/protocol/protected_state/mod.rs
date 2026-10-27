//! Fixed read-only state contracts, separate from authority mutation commands.

/// Unit request envelope.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
/// Compact state and fence evidence, including maximum principal and counter values.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 16_384,
    nodes: 512,
    depth: 16,
};

/// Controller-only Root and Coordinator authority-fence evidence.
pub mod authority_restore {
    pub const METHOD: &str = "canic_authority_restore_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0800;
    pub use super::{REQUEST_LIMITS, RESPONSE_LIMITS};
}

/// Controller-only Root local Fleet state.
pub mod fleet_state {
    pub const METHOD: &str = "canic_root_fleet_state";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0801;
    pub use super::{REQUEST_LIMITS, RESPONSE_LIMITS};
}

#[cfg(test)]
mod tests;

/// Seal one exact authority snapshot operation; this does not authorize release or restore.
pub mod snapshot_prepare {
    pub const METHOD: &str = "canic_authority_snapshot_prepare";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0802;
    pub use super::RESPONSE_LIMITS;
    pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
        bytes: 128,
        allocation: 2048,
        nodes: 256,
        depth: 8,
    };
}
/// Resume only the original live authority operation after its history check.
pub mod snapshot_resume {
    pub const METHOD: &str = "canic_authority_snapshot_resume";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_0803;
    pub use super::snapshot_prepare::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
