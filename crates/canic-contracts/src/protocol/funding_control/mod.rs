//! Typed funding controls retain exact spending and rotation identities.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Exact root accept method.
pub mod root_accept {
    pub const METHOD: &str = "canic_root_accept_funding";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2100;
}
/// Exact root prepare method.
pub mod root_prepare {
    pub const METHOD: &str = "canic_root_prepare_funding_policy_rotation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2101;
}
/// Exact root activate method.
pub mod root_activate {
    pub const METHOD: &str = "canic_root_activate_funding_policy_rotation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2102;
}
/// Exact request method.
pub mod request {
    pub const METHOD: &str = "canic_coordinator_request_root_funding";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2103;
}
/// Exact begin method.
pub mod begin {
    pub const METHOD: &str = "canic_coordinator_begin_funding_policy_rotation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2104;
}
/// Exact stage method.
pub mod stage {
    pub const METHOD: &str = "canic_coordinator_stage_funding_policy_rotation_root";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2105;
}
/// Exact apply method.
pub mod apply {
    pub const METHOD: &str = "canic_coordinator_apply_funding_policy_rotation";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2106;
}
/// Exact set enabled method.
pub mod set_enabled {
    pub const METHOD: &str = "canic_coordinator_set_root_funding";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2107;
}

#[cfg(test)]
mod tests;
