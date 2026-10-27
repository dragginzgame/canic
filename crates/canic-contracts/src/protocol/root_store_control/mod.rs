//! Root-owned Store preparation and publication under the original lifecycle contracts.
pub use crate::protocol::admission_control::{REQUEST_LIMITS, RESPONSE_LIMITS};
/// Exact adopt update.
pub mod adopt {
    pub const METHOD: &str = "canic_root_adopt_store";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2800;
}
/// Exact bootstrap update.
pub mod bootstrap {
    pub const METHOD: &str = "canic_root_bootstrap_store";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2801;
}
/// Exact prepare update.
pub mod prepare {
    pub const METHOD: &str = "canic_root_prepare_store_fixture";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2802;
}
/// Exact publish update.
pub mod publish {
    pub const METHOD: &str = "canic_root_publish_release_set";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_2803;
}

/// Exact retained adoption evidence; no new Store effects.
pub mod adoption {
    pub const METHOD: &str = "canic_root_store_adoption";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3400;
}

/// Exact retained bootstrap evidence; no new Store effects.
pub mod bootstrap_status {
    pub const METHOD: &str = "canic_root_store_bootstrap";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3401;
}

#[cfg(test)]
mod tests;
