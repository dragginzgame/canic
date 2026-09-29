//! Module: view::fleet_activation
//!
//! Responsibility: expose internal activation projections and transition results.
//! Does not own: activation mutation, runtime startup, or endpoint serialization.
//! Boundary: ops reconstructs the role-owned record view and reports committed transitions.

use crate::cdk::types::Principal;
use crate::storage::stable::fleet_activation::{
    ComponentRuntimeRecord, FleetActivationStateRecord, FleetCascadeManifestEntryRecord,
    FleetCredentialManifestRecord, FleetSubnetRootAuthorityRecord,
    FleetSubnetWasmStoreAuthorityRecord,
};
use crate::{
    dto::{
        component_registry::ComponentRuntimeStatusResponse,
        fleet_activation::FleetActivationStatusResponse,
    },
    ids::FleetSubnetWasmStoreActivationAuthority,
};

///
/// FleetActivationView
///
/// Read projection reconstructed by ops from one role-owned activation record.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationView {
    pub state: FleetActivationStateRecord,
    pub root_authority: Option<FleetSubnetRootAuthorityRecord>,
    pub wasm_store_authority: Option<FleetSubnetWasmStoreAuthorityRecord>,
    pub prepared_state_snapshot_hash: Option<[u8; 32]>,
    pub prepared_topology_snapshot_hash: Option<[u8; 32]>,
    pub cascade_manifest: Option<Vec<FleetCascadeManifestEntryRecord>>,
    pub credential_manifests: Vec<FleetCredentialManifestRecord>,
    pub component_runtime: Option<ComponentRuntimeRecord>,
}

/// The exact root-owned Wasm Store included in fresh Fleet activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationWasmStoreView {
    pub pid: Principal,
}

/// Exact retained authority Root uses for its independently installed Store child.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationWasmStoreAuthorityView {
    pub authority: FleetSubnetWasmStoreActivationAuthority,
}

///
/// FleetActivationTransition
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationTransition {
    pub status: FleetActivationStatusResponse,
    pub transitioned: bool,
    pub application_init_args: Option<Vec<u8>>,
}

///
/// ComponentRuntimeActivationTransition
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRuntimeActivationTransition {
    pub status: ComponentRuntimeStatusResponse,
    pub transitioned: bool,
    pub application_init_args: Option<Vec<u8>>,
}
