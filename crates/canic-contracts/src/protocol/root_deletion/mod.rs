//! Controller-owned preparation and completion of one exact observed Root deletion.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Freeze the exact external executor and observed physical authority.
pub mod prepare {
    pub const METHOD: &str = "canic_coordinator_prepare_root_deletion";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2500;
}
/// Record typed absence under that same execution intent.
pub mod complete {
    pub const METHOD: &str = "canic_coordinator_complete_root_deletion";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2501;
}
