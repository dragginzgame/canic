//! Root controller-owned physical pool handoff.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Transfer one reviewed canister to the exact recipient.
pub mod handoff {
    pub const METHOD: &str = "canic_root_handoff_pool_canister";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2600;
}

/// Execute one bounded Root pool maintenance step.
pub mod maintain {
    pub const METHOD: &str = "canic_root_maintain_pool";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2601;
}

/// Import or re-observe the exact physical canister through its existing pool owner.
pub mod import {
    pub const METHOD: &str = "canic_root_import_canister";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2602;
}

#[cfg(test)]
mod tests;
