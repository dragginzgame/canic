//! Preview and durable refill share exact inputs, never an arbitrary-size response.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Observe the cost and estimate without issuing a refill.
pub mod preview {
    pub const METHOD: &str = "canic_root_preview_cycle_refill";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2300;
}
/// Issue or reconcile the exact retained refill operation.
pub mod execute {
    pub const METHOD: &str = "canic_root_refill_cycles";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2301;
}

/// Exact retained operation evidence without advancing its effects.
pub mod status {
    pub const METHOD: &str = "canic_root_cycle_refill_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3501;
}

#[cfg(test)]
mod tests;
