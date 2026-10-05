//! Public identifiers and the passive types needed by their fields and constructors.

pub use crate::__internal::core::ids::{
    AccessMetricKind, AppId, BuildNetwork, COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS, CanisterRole,
    CanonicalNetworkId, CanonicalNetworkIdParseError, CanonicalNetworkTrustAnchorError,
    ComponentBinding, ComponentChildBinding, ComponentDeploymentConfigurationDigest,
    ComponentDeploymentIdParseError, ComponentGroupDeploymentId, ComponentGroupMemberId,
    ComponentGroupMemberPath, ComponentGroupMemberPathError, ComponentGroupPlacementId,
    ComponentGroupSpecId, ComponentInstanceId, ComponentInstanceIdParseError,
    ComponentSpecAdmission, ComponentSpecId, ComponentSpecIdParseError, ComponentTopologyDigest,
    CyclesFundingBudget, EndpointCall, EndpointCallKind, EndpointId, FleetAdmissionPolicy,
    FleetAdmissionPolicyTemplate, FleetAdmissionProjection, FleetAdmissionRule,
    FleetAdmissionSelector, FleetAdmissionTarget, FleetBinding, FleetCoordinatorBinding,
    FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetId, FleetIdParseError, FleetKey,
    FleetName, FleetNameParseError, FleetRegistryAuthority, FleetServiceId,
    FleetSubnetCanisterPoolConfig, FleetSubnetRootAutomaticIcpRefillPolicy, FleetSubnetRootBinding,
    FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy, FleetSubnetRootIcpRefillPolicy,
    FleetSubnetRootLimits, FleetSubnetRootReleaseSet, FleetSubnetWasmStoreActivationAuthority,
    FleetSubnetWasmStoreAuthority, IntentId, IntentResourceKey, ManagedCanisterBinding,
    ReleaseBuildId, ReleaseBuildIdParseError, ReleaseBuildNonce, ReleaseSetDigest, SubnetId, cap,
};

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub use canic_control_plane::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    WasmStoreGcMode, WasmStoreGcStatus,
};
