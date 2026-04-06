pub use crate::__internal::core::ids::{
    AccessMetricKind, BuildNetwork, CanisterRole, EndpointCall, EndpointCallKind, EndpointId,
    IntentResourceKey, SubnetRole, SystemMetricKind, cap,
};

#[cfg(feature = "control-plane")]
pub use canic_control_plane::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    WasmStoreGcMode, WasmStoreGcStatus,
};
