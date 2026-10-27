//! Native transport for retained Coordinator acknowledgement evidence.

use crate::dto::registry_discovery::RegistryVersion;
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// One Root's acknowledgement of an exact durably staged registry snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootAcknowledgement {
    pub fleet_subnet_root: Principal,
    pub version: RegistryVersion,
}

/// Exact registry version acknowledged by the authenticated Root caller.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct AcknowledgeRequest {
    pub version: RegistryVersion,
}
