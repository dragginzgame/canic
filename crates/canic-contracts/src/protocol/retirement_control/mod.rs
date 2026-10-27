//! Native draining and terminal retirement methods retain their original effect owners.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};

/// Exact remove mutation.
pub mod remove {
    pub const METHOD: &str = "canic_coordinator_remove_root";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2900;
}

/// Exact root_remove mutation.
pub mod root_remove {
    pub const METHOD: &str = "canic_root_remove_root";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2901;
}

/// Exact retire mutation.
pub mod retire {
    pub const METHOD: &str = "canic_coordinator_retire";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2902;
}

#[cfg(test)]
mod tests;

/// Exact authorized retained operation observation; no mutation or lease extension.
pub mod status {
    pub const METHOD: &str = "canic_coordinator_retirement_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3302;
}

/// Complete retained Root-removal observation, including uncertain-effect evidence.
pub mod removal_status {
    pub const METHOD: &str = "canic_coordinator_root_removal";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3303;
}
