//! Module: registry
//!
//! Responsibility: define host-observed registry entries for backup planning.
//! Does not own: registry querying, manifest projection, or stable storage.
//! Boundary: data shape consumed by backup discovery and plan construction.

///
/// RegistryEntry
///
/// Host-observed canister registry row used as backup discovery input.
/// Owned by backup registry support and consumed by discovery and planning.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryEntry {
    pub pid: String,
    pub role: Option<String>,
    pub kind: Option<String>,
    pub parent_pid: Option<String>,
    pub module_hash: Option<String>,
}
