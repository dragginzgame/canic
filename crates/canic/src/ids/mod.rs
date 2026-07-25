pub use crate::__internal::core::ids::{
    AccessMetricKind, BuildNetwork, CanisterRole, CanonicalNetworkId, EndpointCall,
    EndpointCallKind, EndpointId, FleetId, FleetKey, IntentResourceKey, SubnetSlotId,
    SystemMetricKind, TreeGroupId, TreeId, TreeSpecId, cap,
};

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub use canic_control_plane::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    WasmStoreGcMode, WasmStoreGcStatus,
};
