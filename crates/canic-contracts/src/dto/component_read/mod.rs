//! Complete component operation observations; lifecycle authority remains in workflow.
/// Complete native RootComponentChildAllocationResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChildAllocation {
    pub last_failure: Option<ChildFailure>,
    pub operation_id: [u8; 32],
    pub component: crate::ids::ComponentInstanceId,
    pub parent_canister_id: ic_platform::Principal,
    pub parent_role: crate::ids::CanisterRole,
    pub child_role: crate::ids::CanisterRole,
    pub child_kind: crate::deployment::ComponentChildKind,
    pub maximum_instances_per_parent: u32,
    pub maximum_descendants: u32,
    pub maximum_registry_bytes: u64,
    pub reserved_against_registry: crate::dto::component_registry::ComponentRegistryHead,
    pub release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub phase: crate::dto::component_registry::RootComponentAllocationPhase,
    pub creation: Option<Creation>,
    pub installation: Option<ChildInstallation>,
}

/// Complete native RootComponentCreationEvidence evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Creation {
    pub wasm_store: ic_platform::Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: crate::cycles::Cycles,
    pub controller: ic_platform::Principal,
    pub canister: Option<ic_platform::Principal>,
}

/// Complete native RootComponentChildInstallEvidence evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChildInstallation {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: crate::dto::component_directory::ComponentChildBinding,
}

/// Complete native RootComponentChildAllocationFailure evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChildFailure {
    pub diagnostic_code: u16,
    pub failed_at_ns: u64,
    pub consecutive_failures: u32,
    pub retry_at_ns: u64,
    pub platform_rejection: Option<String>,
}

/// Complete native RootComponentProvisioningStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Provisioning {
    pub last_failure: Option<crate::dto::provisioning_census::Failure>,
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: crate::dto::registry_discovery::RegistryVersion,
    pub configuration_digest: crate::ids::ComponentDeploymentConfigurationDigest,
    pub fleet_subnet_root: ic_platform::Principal,
    pub estate_funding_required: Option<EstateFunding>,
    pub phase: crate::dto::component_provisioning::RootComponentProvisioningPhase,
    pub placement_count: u32,
    pub component_count: u32,
    pub reserved_component_count: u32,
    pub claimed_component_count: u32,
    pub installed_component_count: u32,
    pub registry_committed_component_count: u32,
    pub published_component_count: u32,
    pub activated_component_count: u32,
    pub root_runtime_active: bool,
    pub result: Option<ProvisioningResult>,
    pub publication: Option<Publication>,
    pub activation: Option<crate::dto::component_provisioning::RootComponentActivationEvidence>,
    pub accepted_at_ns: u64,
    pub provisioned_at_ns: Option<u64>,
    pub published_at_ns: Option<u64>,
    pub activation_started_at_ns: Option<u64>,
    pub runtimes_activated_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
}

/// Complete native RootComponentProvisioningResult evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ProvisioningResult {
    pub placements: Vec<Placement>,
}

/// Complete native RootProvisionedGroupPlacement evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Placement {
    pub group_placement: crate::ids::ComponentGroupPlacementId,
    pub component_group: crate::ids::ComponentGroupSpecId,
    pub members: Vec<Member>,
}

/// Complete native RootProvisionedGroupMember evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Member {
    pub member_operation_id: [u8; 32],
    pub member_path: crate::ids::ComponentGroupMemberPath,
    pub component_spec: crate::ids::ComponentSpecId,
    pub purpose: crate::deployment::ComponentDeploymentPurpose,
    pub limits: crate::deployment::ComponentDeploymentLimits,
    pub binding: crate::dto::component_directory::ComponentBinding,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
}

/// Complete native RootComponentPublicationEvidence evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Publication {
    pub fleet_registry: crate::dto::registry_discovery::RegistryVersion,
    pub fleet_directory_content_hash: [u8; 32],
    pub component_directories:
        Vec<crate::dto::component_provisioning::ComponentDirectoryPublicationEvidence>,
    pub component_group_directories:
        Vec<crate::dto::component_provisioning::ComponentGroupDirectoryPublicationEvidence>,
}

/// Complete native RootEstateFundingRequired evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct EstateFunding {
    pub available: crate::cycles::Cycles,
    pub attempt_count: u32,
    pub creation_amount: crate::cycles::Cycles,
    pub cycles_ledger: ic_platform::Principal,
    pub execution_margin: crate::cycles::Cycles,
    pub last_attempt_at_ns: Option<u64>,
    pub ledger_fee: crate::cycles::Cycles,
    pub management_creation_fee: crate::cycles::Cycles,
    pub operation_id: [u8; 32],
    pub readiness_floor: crate::cycles::Cycles,
    pub required: crate::cycles::Cycles,
    pub retry_at_ns: u64,
    pub root: ic_platform::Principal,
    pub shortfall: crate::cycles::Cycles,
}

/// Complete native FleetComponentProvisioningRootProgress observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorRootProgress {
    pub fleet_subnet_root: ic_platform::Principal,
    pub component_count: u32,
    pub reserved_component_count: u32,
    pub claimed_component_count: u32,
    pub installed_component_count: u32,
    pub registry_committed_component_count: u32,
}

/// Complete native FleetComponentPublicationRootProgress observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorPublicationProgress {
    pub fleet_subnet_root: ic_platform::Principal,
    pub component_count: u32,
    pub published_component_count: u32,
}

/// Complete native FleetComponentSynchronizationRootProgress observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorSynchronizationProgress {
    pub fleet_subnet_root: ic_platform::Principal,
    pub affected_component_count: u32,
    pub synchronized_component_count: u32,
    pub complete: bool,
}

/// Complete native FleetComponentActivationRootProgress observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorActivationProgress {
    pub fleet_subnet_root: ic_platform::Principal,
    pub component_count: u32,
    pub activated_component_count: u32,
    pub root_runtime_active: bool,
}

/// Complete native FleetComponentProvisioningRootFailure observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorFailure {
    pub origin: Option<FailureOrigin>,
    pub fleet_subnet_root: ic_platform::Principal,
    pub stage: crate::dto::component_provisioning::FleetComponentProvisioningRetryStage,
    pub diagnostic_code: u16,
    pub failed_at_ns: u64,
}

/// Complete native FleetComponentProvisioningStatusResponse observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorProvisioning {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: crate::dto::registry_discovery::RegistryVersion,
    pub configuration_digest: crate::ids::ComponentDeploymentConfigurationDigest,
    pub operation: crate::dto::component_provisioning::FleetComponentProvisioningOperation,
    pub phase: crate::dto::component_provisioning::FleetComponentProvisioningPhase,
    pub directory_confirmation_root_count: u32,
    pub root_batch_count: u32,
    pub accepted_root_count: u32,
    pub acceptance_in_flight_root: Option<ic_platform::Principal>,
    pub provisioned_root_count: u32,
    pub current_root: Option<CoordinatorRootProgress>,
    pub provisioning_in_flight_root: Option<ic_platform::Principal>,
    pub directory_confirmed_root_count: u32,
    pub current_synchronization: Option<CoordinatorSynchronizationProgress>,
    pub current_publication: Option<CoordinatorPublicationProgress>,
    pub publication_in_flight_root: Option<ic_platform::Principal>,
    pub runtime_activated_root_count: u32,
    pub current_activation: Option<CoordinatorActivationProgress>,
    pub activation_in_flight_root: Option<ic_platform::Principal>,
    pub pending_root_failure: Option<CoordinatorFailure>,
    pub estate_funding_required: Option<EstateFunding>,
    pub group_placement_count: u32,
    pub component_count: u32,
    pub planned_at_ns: u64,
    pub roots_accepted_at_ns: Option<u64>,
    pub components_provisioned_at_ns: Option<u64>,
    pub published_fleet_registry: Option<crate::dto::registry_discovery::RegistryVersion>,
    pub service_topology_published_at_ns: Option<u64>,
    pub directories_confirmed_at_ns: Option<u64>,
    pub runtimes_activated_at_ns: Option<u64>,
}

/// Original failure authority without receiver-owned retry counters.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FailureOrigin {
    pub dependency: Option<crate::dto::provisioning_census::DependencyFailure>,
    pub failed_at_ns: u64,
    pub retry_at_ns: Option<u64>,
    pub stage: crate::values::provisioning_failure::ProvisioningFailureStage,
    pub target: ic_platform::Principal,
    pub operation_id: [u8; 32],
    pub diagnostic_code: u16,
    pub retry_category: crate::values::provisioning_failure::ProvisioningRetryCategory,
}

/// Complete top-level allocation and completion evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentAllocation {
    pub allocation: Allocation,
    pub complete: bool,
}

/// Exact reservation, causal authority and retained installation evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Allocation {
    pub application_initialization:
        Option<crate::dto::component_initialization::ComponentApplicationInitialization>,
    pub operation_id: [u8; 32],
    pub allocation_sequence: u64,
    pub component: crate::ids::ComponentInstanceId,
    pub component_spec: crate::ids::ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: crate::ids::CanisterRole,
    pub provisioning_origin: crate::dto::registry_read::ProvisioningOrigin,
    pub release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub phase: crate::dto::component_registry::RootComponentAllocationPhase,
    pub creation: Option<Creation>,
    pub installation: Option<Installation>,
}

/// Exact component installation identity and chunk evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Installation {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: crate::dto::component_directory::ComponentBinding,
}
