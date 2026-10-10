//! Public identifiers and the passive types needed by their fields and constructors.

pub use canic_contracts::ids::{
    AccessMetricKind, AppId, BuildNetwork, COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS,
    CallerComponentInstallation, CallerInstallation, CallerReceiverAuthority, CallerRootAuthority,
    CanisterRole, CanonicalNetworkId, CanonicalNetworkIdParseError,
    CanonicalNetworkTrustAnchorError, ComponentBinding, ComponentChildBinding,
    ComponentDeploymentConfigurationDigest, ComponentDeploymentIdParseError,
    ComponentGroupDeploymentId, ComponentGroupMemberId, ComponentGroupMemberPath,
    ComponentGroupMemberPathError, ComponentGroupPlacementId, ComponentGroupSpecId,
    ComponentInstanceId, ComponentInstanceIdParseError, ComponentSpecAdmission, ComponentSpecId,
    ComponentSpecIdParseError, ComponentTopologyDigest, CyclesFundingBudget, EndpointCall,
    EndpointCallKind, EndpointId, FleetAdmissionPolicy, FleetAdmissionPolicyTemplate,
    FleetAdmissionProjection, FleetAdmissionRule, FleetAdmissionSelector, FleetAdmissionTarget,
    FleetBinding, FleetCoordinatorBinding, FleetCoordinatorRootFundingPolicy, FleetFundingProfile,
    FleetId, FleetIdParseError, FleetKey, FleetName, FleetNameParseError, FleetRegistryAuthority,
    FleetServiceId, FleetSubnetCanisterPoolConfig, FleetSubnetRootAutomaticIcpRefillPolicy,
    FleetSubnetRootBinding, FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy,
    FleetSubnetRootIcpRefillPolicy, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
    FleetSubnetWasmStoreActivationAuthority, FleetSubnetWasmStoreAuthority, ManagedCanisterBinding,
    ReleaseBuildId, ReleaseBuildIdParseError, ReleaseBuildNonce, ReleaseSetDigest, SubnetId, cap,
};

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub use canic_contracts::{
    ids::TemplateChunkingMode, ids::TemplateId, ids::TemplateManifestState, ids::TemplateVersion,
    ids::WasmStoreBinding, ids::WasmStoreGcMode, ids::WasmStoreGcStatus,
};

pub use canic_core::{ids::IntentId, ids::IntentResourceKey};
