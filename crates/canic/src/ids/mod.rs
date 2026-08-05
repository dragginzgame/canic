pub use crate::__internal::core::ids::{
    AccessMetricKind, BuildNetwork, COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS, CanisterRole,
    CanonicalNetworkId, ComponentBinding, ComponentChildBinding, ComponentGroupDeploymentId,
    ComponentGroupMemberId, ComponentGroupMemberPath, ComponentGroupPlacementId,
    ComponentGroupSpecId, ComponentInstanceId, ComponentSpecAdmission, ComponentSpecId,
    ComponentTopologyDigest, CyclesFundingBudget, EndpointCall, EndpointCallKind, EndpointId,
    FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
    FleetServiceId, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootLimits,
    FleetSubnetRootReleaseSet, FleetSubnetWasmStoreAuthority, IntentResourceKey,
    ManagedCanisterBinding, ReleaseSetDigest, SubnetId, SystemMetricKind, cap,
};

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub use canic_control_plane::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    WasmStoreGcMode, WasmStoreGcStatus,
};
