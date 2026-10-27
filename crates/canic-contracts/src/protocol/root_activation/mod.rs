//! Root preparation and resume retain one activation identity and credential generation.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
pub mod prepare {
    pub const METHOD: &str = "canic_root_prepare_activation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2000;
}
pub mod resume {
    pub const METHOD: &str = "canic_root_resume_activation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2001;
}
#[cfg(test)]
mod tests;

/// Exact retained operation evidence without advancing its effects.
pub mod status {
    pub const METHOD: &str = "canic_root_activation_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3500;
}
