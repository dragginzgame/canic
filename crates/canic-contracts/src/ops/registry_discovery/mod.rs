//! Lossless native registry projections; canonical storage and hashing are unchanged.
use crate::dto::{fleet_registry as stored, registry_discovery as wire};

impl From<stored::FleetRegistryVersion> for wire::RegistryVersion {
    fn from(value: stored::FleetRegistryVersion) -> Self {
        Self {
            authority: value.authority.into(),
            revision: value.revision,
            content_hash: value.content_hash,
        }
    }
}
impl From<wire::RegistryVersion> for stored::FleetRegistryVersion {
    fn from(value: wire::RegistryVersion) -> Self {
        Self {
            authority: value.authority.into(),
            revision: value.revision,
            content_hash: value.content_hash,
        }
    }
}
impl From<stored::FleetRegistryManifest> for wire::RegistryManifest {
    fn from(value: stored::FleetRegistryManifest) -> Self {
        Self {
            authority: value.authority.into(),
            revision: value.revision,
            byte_length: value.byte_length,
            content_hash: value.content_hash,
        }
    }
}
impl From<wire::RegistryManifest> for stored::FleetRegistryManifest {
    fn from(value: wire::RegistryManifest) -> Self {
        Self {
            authority: value.authority.into(),
            revision: value.revision,
            byte_length: value.byte_length,
            content_hash: value.content_hash,
        }
    }
}
