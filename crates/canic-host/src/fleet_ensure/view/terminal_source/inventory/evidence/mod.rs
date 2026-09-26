//! Module: fleet_ensure::view::terminal_source::inventory::evidence
//!
//! Responsibility: project historical source membership and finalized release evidence.
//! Does not own: current executable contracts, defaults, conversion or IC interfaces.
//! Boundary: omitted declaration fields remain bound by the separate receipt audit.

use super::super::receipt_evidence::EvidenceFleetRegistryAuthority;
use crate::fleet_ensure::model::{
    DesiredCanister, DesiredFleetBootstrapRoot, FleetEnsureTopologyRecord,
};
use canic_core::{
    dto::fleet_registry::FleetSubnetRootEntry,
    ids::{AppId, BuildNetwork, CanonicalNetworkId, FleetId, ReleaseBuildId, SubnetId},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identity-only declaration projection. The receipt owner binds every omitted field.
#[derive(Deserialize)]
pub(in crate::fleet_ensure) struct InventoryDeclarationEvidence {
    pub(in crate::fleet_ensure) bootstrap: InventoryBootstrapEvidence,
    pub(in crate::fleet_ensure) canisters: Vec<DesiredCanister>,
}

/// Historical physical placement without a recovery-controller declaration.
#[derive(Deserialize)]
pub(in crate::fleet_ensure) struct InventoryBootstrapEvidence {
    pub(in crate::fleet_ensure) app: AppId,
    pub(in crate::fleet_ensure) canonical_network_id: CanonicalNetworkId,
    pub(in crate::fleet_ensure) coordinator: String,
    pub(in crate::fleet_ensure) coordinator_subnet: SubnetId,
    pub(in crate::fleet_ensure) fleet_id: FleetId,
    pub(in crate::fleet_ensure) release_build_id: ReleaseBuildId,
    pub(in crate::fleet_ensure) roots: Vec<DesiredFleetBootstrapRoot>,
}

/// Source registry identity and Root membership claims, not a current Registry.
#[derive(Deserialize)]
pub(in crate::fleet_ensure) struct RegistryInventoryEvidence {
    pub(in crate::fleet_ensure) authority: EvidenceFleetRegistryAuthority,
    pub(in crate::fleet_ensure) revision: u64,
    pub(in crate::fleet_ensure) fleet_subnet_roots: Vec<FleetSubnetRootEntry>,
}

/// Historical state has no conversion to current execution state.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct CompletedStateEvidence {
    pub(in crate::fleet_ensure) active_registry: RegistryInventoryEvidence,
    pub(in crate::fleet_ensure) completed_reinstall_action_sha256: BTreeMap<String, String>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub(in crate::fleet_ensure) completed_reinstall_operation_id: Option<String>,
    pub(in crate::fleet_ensure) completed_reinstalls: BTreeMap<String, u64>,
    pub(in crate::fleet_ensure) fleet: String,
    pub(in crate::fleet_ensure) pending_principals: BTreeMap<String, String>,
    pub(in crate::fleet_ensure) principals: BTreeMap<String, String>,
    pub(in crate::fleet_ensure) retained_cycles_by_principal: BTreeMap<String, u128>,
    pub(in crate::fleet_ensure) schema_version: u16,
    pub(in crate::fleet_ensure) topology: BTreeMap<String, FleetEnsureTopologyRecord>,
}

/// Frozen manifest evidence, never a current release manifest or transition policy.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct CompletedReleaseManifestEvidence {
    pub(in crate::fleet_ensure) application_artifact_union_sha256: [u8; 32],
    pub(in crate::fleet_ensure) build_network: BuildNetwork,
    pub(in crate::fleet_ensure) fixture_artifact_manifest_sha256: [u8; 32],
    pub(in crate::fleet_ensure) infrastructure_artifact_manifest_sha256: [u8; 32],
    pub(in crate::fleet_ensure) release_build_id: ReleaseBuildId,
    pub(in crate::fleet_ensure) schema_version: u16,
}
