//! Root allocation/removal methods with exact operation receipts.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Exact provision child method.
pub mod child {
    pub const METHOD: &str = "canic_root_provision_child";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2400;
}
/// Exact provision peer method.
pub mod peer {
    pub const METHOD: &str = "canic_root_provision_peer";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2401;
}
/// Exact remove component method.
pub mod remove {
    pub const METHOD: &str = "canic_root_remove_component";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2402;
}
/// Exact remove subtree method.
pub mod subtree {
    pub const METHOD: &str = "canic_root_remove_subtree";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2403;
}

/// Reserve one controller-selected top-level Component with its original operation ID.
pub mod provision {
    pub const METHOD: &str = "canic_root_provision_component";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2404;
}

/// Prepare an empty Root registry against exact reviewed Fleet and Store authority.
pub mod prepare {
    pub const METHOD: &str = "canic_root_prepare_component_registry";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2405;
}
