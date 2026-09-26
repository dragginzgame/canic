//! Module: fleet_ensure::view::terminal_source::receipt_evidence
//!
//! Responsibility: preserve the 0.110.38 declaration order for completed receipt hashing.
//! Does not own: Candid interfaces, current authority, execution, migration or defaults.
//! Boundary: these frozen evidence projections have no conversion into executable plans.
//!
//! Only the authority-containing shapes are projected here. Unchanged passive
//! value types retain their owning definitions; exact retained hashes detect drift.

use crate::fleet_ensure::model::{
    DesiredCanister, DesiredFleetBootstrapRoot, DesiredFleetProtocol,
};
use candid::Principal;
use canic_core::{
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, FleetComponentProvisioningOperation,
        },
        component_registry::RootComponentInitialInventoryStatus,
        fleet_registry::{
            FleetComponentSpecEntry, FleetDirectoryService, FleetServiceBinding,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootEntry,
        },
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentSpecAdmission, ComponentTopologyDigest,
        FleetAdmissionPolicy, FleetBinding, FleetSubnetRootFundingAuthority, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, SubnetId,
    },
};
use serde::{Deserialize, Serialize};

/// Completed protocol receipt payloads; historical field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
#[expect(
    clippy::large_enum_variant,
    reason = "historical evidence is boxed by the receipt action envelope"
)]
pub(in crate::fleet_ensure) enum CompletedProtocolEvidence {
    ObservePoolReadiness {
        minimum_ready: u32,
        readiness_floor: canic_core::cdk::types::Cycles,
    },
    MaintainPoolReadiness {
        maximum_updates: u32,
        minimum_ready: u32,
        readiness_floor: canic_core::cdk::types::Cycles,
    },
    ReconcilePoolAsset {
        request: canic_core::dto::pool::PoolCanisterRequest,
        minimum_cycles: canic_core::cdk::types::Cycles,
    },
    ActivateRegistry {
        expected_registry: EvidenceFleetRegistry,
        expected_version: EvidenceFleetRegistryVersion,
        request: EvidenceFleetRegistryActivationRequest,
    },
    ActivateRegistryMirror {
        expected: EvidenceFleetSubnetRootRegistryMirrorActivationResponse,
        request: EvidenceFleetSubnetRootRegistrySyncRequest,
    },
    AdoptStore {
        request: EvidenceFleetSubnetWasmStoreAdoptionRequest,
    },
    BootstrapStore {
        expected: canic_core::dto::root_store::RootStoreBootstrapResponse,
        request: canic_core::dto::root_store::RootStoreBootstrapRequest,
    },
    JoinRoot {
        expected_registry: EvidenceFleetRegistry,
        expected_version: EvidenceFleetRegistryVersion,
        request: EvidenceFleetSubnetRootJoinRequest,
    },
    PrepareStoreFixture {
        maximum_attempts: u32,
        request: canic_core::dto::root_store::RootStoreFixturePrepareRequest,
        source: canic_core::dto::root_store::RootStoreFixture,
        store: candid::Principal,
    },
    PublishStoreFixtureChunk {
        maximum_attempts: u32,
        request: canic_core::dto::fixture_provisioning::FixtureChunkUpload,
        expected: canic_core::dto::fixture_provisioning::FixtureSourceStatus,
        source_bytes: u64,
    },
    PrepareComponentRegistry {
        expected: EvidenceRootComponentRegistryStatusResponse,
        request: EvidenceRootComponentRegistryPreparationRequest,
    },
    ProvisionComponents {
        plan_hash: [u8; 32],
        request: EvidenceFleetComponentProvisioningPrepareRequest,
    },
    PublishStoreChunk {
        request: canic_control_plane::dto::template::TemplateChunkInput,
    },
    SynchronizeRegistry {
        expected: EvidenceFleetSubnetRootRegistrySyncResponse,
        request: EvidenceFleetSubnetRootRegistrySyncRequest,
    },
}

/// Frozen historical DesiredFleet evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceDesiredFleet {
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub(in crate::fleet_ensure) bootstrap: Option<EvidenceDesiredFleetBootstrap>,
    pub(in crate::fleet_ensure) canisters: Vec<DesiredCanister>,
    pub(in crate::fleet_ensure) cycles_ledger: String,
    pub(in crate::fleet_ensure) environment: String,
    pub(in crate::fleet_ensure) fleet: String,
    pub(in crate::fleet_ensure) ledger_fee_cycles: String,
    pub(in crate::fleet_ensure) management_creation_fee_cycles: String,
    pub(in crate::fleet_ensure) material_cycle_threshold: String,
    pub(in crate::fleet_ensure) maximum_observation_burn_cycles: String,
    pub(in crate::fleet_ensure) maximum_stalled_observations: u32,
    pub(in crate::fleet_ensure) maximum_update_burn_cycles: String,
    pub(in crate::fleet_ensure) operator: String,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub(in crate::fleet_ensure) protocol: Option<DesiredFleetProtocol>,
    pub(in crate::fleet_ensure) schema_version: u16,
    pub(in crate::fleet_ensure) treasury: String,
}

/// Frozen historical DesiredFleetBootstrap evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceDesiredFleetBootstrap {
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub(in crate::fleet_ensure) admission_identity_origin: Option<String>,
    pub(in crate::fleet_ensure) admission: canic_core::ids::FleetAdmissionPolicyTemplate,
    pub(in crate::fleet_ensure) app: canic_core::ids::AppId,
    pub(in crate::fleet_ensure) canonical_network_id: canic_core::ids::CanonicalNetworkId,
    pub(in crate::fleet_ensure) component_deployment_configuration:
        canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    pub(in crate::fleet_ensure) coordinator: String,
    pub(in crate::fleet_ensure) coordinator_subnet: canic_core::ids::SubnetId,
    pub(in crate::fleet_ensure) fleet_id: canic_core::ids::FleetId,
    pub(in crate::fleet_ensure) fresh_estate: bool,
    pub(in crate::fleet_ensure) release_build_id: canic_core::ids::ReleaseBuildId,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub(in crate::fleet_ensure) root_funding:
        Option<canic_core::ids::FleetCoordinatorRootFundingPolicy>,
    pub(in crate::fleet_ensure) roots: Vec<DesiredFleetBootstrapRoot>,
}

/// Frozen historical FleetComponentProvisioningPlan evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetComponentProvisioningPlan {
    pub(in crate::fleet_ensure) fleet: FleetBinding,
    pub(in crate::fleet_ensure) fleet_registry: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) configuration_digest: ComponentDeploymentConfigurationDigest,
    pub(in crate::fleet_ensure) operation: FleetComponentProvisioningOperation,
    pub(in crate::fleet_ensure) directory_confirmation_roots: Vec<Principal>,
    pub(in crate::fleet_ensure) batches: Vec<EvidenceFleetSubnetRootProvisioningBatch>,
}

/// Frozen historical FleetComponentProvisioningPrepareRequest evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetComponentProvisioningPrepareRequest {
    pub(in crate::fleet_ensure) operation_id: [u8; 32],
    pub(in crate::fleet_ensure) plan: EvidenceFleetComponentProvisioningPlan,
}

/// Frozen historical FleetCoordinatorBinding evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetCoordinatorBinding {
    pub(in crate::fleet_ensure) fleet: FleetBinding,
    pub(in crate::fleet_ensure) coordinator_subnet: SubnetId,
    pub(in crate::fleet_ensure) coordinator: Principal,
}

/// Frozen historical FleetDirectoryProvenance evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetDirectoryProvenance {
    pub(in crate::fleet_ensure) registry: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) source_fleet_subnet_root: Principal,
}

/// Frozen historical FleetDirectorySnapshot evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetDirectorySnapshot {
    pub(in crate::fleet_ensure) provenance: EvidenceFleetDirectoryProvenance,
    pub(in crate::fleet_ensure) fleet_subnet_roots: Vec<FleetSubnetRootDirectoryEntry>,
    pub(in crate::fleet_ensure) services: Vec<FleetDirectoryService>,
}

/// Frozen historical FleetRegistry evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetRegistry {
    pub(in crate::fleet_ensure) authority: EvidenceFleetRegistryAuthority,
    pub(in crate::fleet_ensure) revision: u64,
    pub(in crate::fleet_ensure) admission: FleetAdmissionPolicy,
    pub(in crate::fleet_ensure) component_specs: Vec<FleetComponentSpecEntry>,
    pub(in crate::fleet_ensure) fleet_subnet_roots: Vec<FleetSubnetRootEntry>,
    pub(in crate::fleet_ensure) services: Vec<FleetServiceBinding>,
}

/// Frozen historical FleetRegistryActivationRequest evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetRegistryActivationRequest {
    pub(in crate::fleet_ensure) expected_registry: EvidenceFleetRegistryVersion,
}

/// Frozen historical FleetRegistryAuthority evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetRegistryAuthority {
    pub(in crate::fleet_ensure) binding: EvidenceFleetCoordinatorBinding,
    pub(in crate::fleet_ensure) epoch: u64,
}

/// Frozen historical FleetRegistryVersion evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetRegistryVersion {
    pub(in crate::fleet_ensure) authority: EvidenceFleetRegistryAuthority,
    pub(in crate::fleet_ensure) revision: u64,
    pub(in crate::fleet_ensure) content_hash: [u8; 32],
}

/// Frozen historical FleetSubnetRootBinding evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootBinding {
    pub(in crate::fleet_ensure) authority: EvidenceFleetRegistryAuthority,
    pub(in crate::fleet_ensure) placement_subnet: SubnetId,
    pub(in crate::fleet_ensure) fleet_subnet_root: Principal,
    pub(in crate::fleet_ensure) component_admissions: Vec<ComponentSpecAdmission>,
    pub(in crate::fleet_ensure) component_topology_digest: ComponentTopologyDigest,
    pub(in crate::fleet_ensure) limits: FleetSubnetRootLimits,
    pub(in crate::fleet_ensure) funding: FleetSubnetRootFundingAuthority,
}

/// Frozen historical FleetSubnetRootJoinRequest evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootJoinRequest {
    pub(in crate::fleet_ensure) expected_registry: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) entry: FleetSubnetRootEntry,
}

/// Frozen historical FleetSubnetRootProvisioningBatch evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootProvisioningBatch {
    pub(in crate::fleet_ensure) root: EvidenceFleetSubnetRootBinding,
    pub(in crate::fleet_ensure) active_release_set: FleetSubnetRootReleaseSet,
    pub(in crate::fleet_ensure) placements: Vec<ComponentGroupPlacementPlan>,
}

/// Frozen historical FleetSubnetRootRegistryMirrorActivationResponse evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootRegistryMirrorActivationResponse {
    pub(in crate::fleet_ensure) fleet_subnet_root: Principal,
    pub(in crate::fleet_ensure) previous_registry: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) version: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) directory: EvidenceFleetDirectorySnapshot,
}

/// Frozen historical FleetSubnetRootRegistrySyncRequest evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootRegistrySyncRequest {
    pub(in crate::fleet_ensure) operation_id: [u8; 32],
    pub(in crate::fleet_ensure) expected_registry: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) store_bootstrap: RootStoreBootstrapRequest,
}

/// Frozen historical FleetSubnetRootRegistrySyncResponse evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootRegistrySyncResponse {
    pub(in crate::fleet_ensure) fleet_subnet_root: Principal,
    pub(in crate::fleet_ensure) version: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) acknowledgement: EvidenceFleetSubnetRootSnapshotAcknowledgement,
}

/// Frozen historical FleetSubnetRootSnapshotAcknowledgement evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetRootSnapshotAcknowledgement {
    pub(in crate::fleet_ensure) fleet_subnet_root: Principal,
    pub(in crate::fleet_ensure) version: EvidenceFleetRegistryVersion,
}

/// Frozen historical FleetSubnetWasmStoreAdoptionRequest evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetWasmStoreAdoptionRequest {
    pub(in crate::fleet_ensure) operation_id: [u8; 32],
    pub(in crate::fleet_ensure) authority: EvidenceFleetSubnetWasmStoreAuthority,
}

/// Frozen historical FleetSubnetWasmStoreAuthority evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceFleetSubnetWasmStoreAuthority {
    pub(in crate::fleet_ensure) authority: EvidenceFleetRegistryAuthority,
    pub(in crate::fleet_ensure) placement_subnet: SubnetId,
    pub(in crate::fleet_ensure) fleet_subnet_root: Principal,
    pub(in crate::fleet_ensure) wasm_store: Principal,
    pub(in crate::fleet_ensure) installation_controller: Principal,
    pub(in crate::fleet_ensure) release_build_id: ReleaseBuildId,
    pub(in crate::fleet_ensure) wasm_module_hash: [u8; 32],
}

/// Frozen historical RootComponentRegistryPreparationRequest evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceRootComponentRegistryPreparationRequest {
    pub(in crate::fleet_ensure) store_bootstrap: RootStoreBootstrapRequest,
    pub(in crate::fleet_ensure) expected_fleet_registry: EvidenceFleetRegistryVersion,
}

/// Frozen historical RootComponentRegistryStatusResponse evidence; field order is hash-significant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct EvidenceRootComponentRegistryStatusResponse {
    pub(in crate::fleet_ensure) fleet_subnet_root: Principal,
    pub(in crate::fleet_ensure) prepared_against_registry: EvidenceFleetRegistryVersion,
    pub(in crate::fleet_ensure) release_set: FleetSubnetRootReleaseSet,
    pub(in crate::fleet_ensure) component_topology_digest: ComponentTopologyDigest,
    pub(in crate::fleet_ensure) next_allocation_sequence: u64,
    pub(in crate::fleet_ensure) reserved_component_instances: u32,
    pub(in crate::fleet_ensure) committed_component_instances: u32,
    pub(in crate::fleet_ensure) managed_descendants: u32,
    pub(in crate::fleet_ensure) known_created_component_canisters: u32,
    pub(in crate::fleet_ensure) encoded_bytes: u64,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub(in crate::fleet_ensure) initial_inventory: Option<RootComponentInitialInventoryStatus>,
}

/// Historical reviewed input, retained exclusively to verify completed receipt hashes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct CompletedDesiredEvidence {
    pub desired: EvidenceDesiredFleet,
    pub protocol_steps: Vec<crate::fleet_ensure::model::DesiredProtocolStep>,
}

/// Closed completed effect evidence. It cannot be passed to the effect driver.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub(in crate::fleet_ensure) enum CompletedActionEvidence {
    Fund {
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        pool_funding: Option<()>,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        amount: u128,
        created_at_time: u64,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        expected_post_cycles: u128,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        funding_deficit_cycles: u128,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        funding_margin_cycles: u128,
        ledger: String,
        name: String,
        principal: String,
    },
    Install {
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        canic_init: Option<crate::fleet_ensure::model::DesiredCanisterInit>,
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        reinstall_witness: Option<()>,
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        init_arg: Option<String>,
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        init_arg_sha256: Option<String>,
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        init_candid: Option<String>,
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        init_candid_sha256: Option<String>,
        mode: crate::fleet_ensure::model::InstallMode,
        name: String,
        principal: String,
        wasm: String,
        wasm_sha256: String,
    },
    FleetProtocol {
        action: Box<CompletedProtocolEvidence>,
        candid: String,
        candid_sha256: String,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        maximum_execution_burn_cycles: u128,
        name: String,
        principal: String,
    },
}

/// Per-canister historical receipt sequence, with no execution methods.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct CompletedCanisterEvidence {
    pub actions: Vec<CompletedActionEvidence>,
    pub disposition: crate::fleet_ensure::model::CanisterDisposition,
    pub name: String,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_cycles: u128,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub principal: Option<String>,
}

/// Hash projection of a completed historical phase. Unsupported authority stays unrepresentable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct CompletedPhaseEvidence {
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub recovery_review: Option<Box<crate::fleet_ensure::model::FleetRecoveryReview>>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub reinstall: Option<()>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub continuation: Option<crate::fleet_ensure::model::FleetEnsureContinuationAuthority>,
    pub canisters: Vec<CompletedCanisterEvidence>,
    pub conservation: crate::fleet_ensure::model::CycleConservation,
    pub desired_sha256: String,
    pub environment: String,
    pub fleet: String,
    pub operation_id: String,
    pub plan_sha256: String,
    pub planned_at_time: u64,
    pub protocol_actions: Vec<CompletedActionEvidence>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub root_start_authority: Option<()>,
    pub root_reinstall_bindings: Vec<()>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub reviewed_desired: Option<CompletedDesiredEvidence>,
    pub schema_version: u16,
    pub scope: crate::fleet_ensure::model::FleetEnsurePlanScope,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub terminal_inventory_operation_id: Option<String>,
}
