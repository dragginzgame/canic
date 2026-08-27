//! Module: registry
//!
//! Responsibility: define host-side canister topology entries.
//! Does not own: topology discovery, transport, or deployment policy.
//! Boundary: shared projection consumed by host and operator workflows.

///
/// RegistryEntry
///

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub pid: String,
    pub role: Option<String>,
    pub parent_pid: Option<String>,
    pub module_hash: Option<String>,
    pub protocol_binding: Option<crate::protocol_binding::RegistryProtocolBinding>,
}
