//! Module: protocol_binding
//!
//! Responsibility: validate exact host-side protocol bindings before Canister transport.
//! Does not own: Fleet discovery, artifact production, or live Overview negotiation.
//! Boundary: retained Wire schemas and compiled profiles must match immutable release evidence.

mod installed;
mod release;

pub use crate::artifact_io::{WireArtifactError, require_wire_method};
pub use installed::{InstalledDeclarationError, load_installed_wire_schema};

use canic_contracts::ids::CanisterRole;
use canic_core::role_contract::{ProtocolProfileDigest, RoleCapabilityKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub use release::{
    ReleaseProtocolBindingError, resolve_current_registry_wire_schema,
    resolve_infrastructure_wire_profile, resolve_release_registry_wire_schema,
};
/// Complete immutable protocol identity selected before one role call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryProtocolBinding {
    pub release_identity: String,
    pub role: CanisterRole,
    pub capabilities: BTreeSet<RoleCapabilityKey>,
    pub wire_schema_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}
