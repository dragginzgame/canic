pub use crate::__internal::core::ids::{
    AccessMetricKind, BuildNetwork, CanisterRole, CanonicalNetworkId, ComponentBinding,
    ComponentChildBinding, ComponentInstanceId, ComponentSpecAdmission, ComponentSpecId,
    ComponentTopologyDigest, CyclesFundingBudget, EndpointCall, EndpointCallKind, EndpointId,
    FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
    FleetSubnetRootBinding, FleetSubnetRootLimits, IntentResourceKey, ReleaseSetDigest, SubnetId,
    SystemMetricKind, cap,
};

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub use canic_control_plane::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    WasmStoreGcMode, WasmStoreGcStatus,
};
