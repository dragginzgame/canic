//! Native Coordinator registry discovery; persisted registry formats remain separate.
use crate::dto::component_directory::FleetRegistryAuthority;
use serde::{Deserialize, Serialize};

/// Exact registry identity; persisted and hashed registry records retain their owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RegistryVersion {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub content_hash: [u8; 32],
}

/// Exact manifest of the canonical registry snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RegistryManifest {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub byte_length: u64,
    pub content_hash: [u8; 32],
}
