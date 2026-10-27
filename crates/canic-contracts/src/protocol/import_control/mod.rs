//! Exact capacity import step methods; retries retain the original sequence and authority.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};

/// Exact reserve import step.
pub mod reserve {
    pub const METHOD: &str = "canic_root_import_reserve";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2c00;
}

/// Exact advance import step.
pub mod advance {
    pub const METHOD: &str = "canic_root_import_advance";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2c01;
}

/// Exact settle import step.
pub mod settle {
    pub const METHOD: &str = "canic_root_import_settle";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2c02;
}

/// Exact release import step.
pub mod release {
    pub const METHOD: &str = "canic_root_import_release";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2c03;
}

#[cfg(test)]
mod tests;
