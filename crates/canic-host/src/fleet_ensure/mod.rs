//! Module: fleet_ensure
//!
//! Responsibility: expose the sole current-generation desired-state Fleet reconciler.
//! Does not own: historical install plans, migration, retained-repair compatibility, or runtime policy.
//! Boundary: current desired state plus live IC observation are the only planning authorities.

pub mod dto;
mod generate;
mod inventory;
pub mod model;
pub mod ops;
pub mod policy;
pub mod workflow;

#[cfg(test)]
mod tests;

pub use dto::{DesiredFleetLoadError, LoadedDesiredFleet, load_desired_fleet};
pub use generate::{
    FleetGenerateError, FleetGenerateRequest, GeneratedDesiredFleet, generate_desired_fleet,
};
pub use inventory::{
    CurrentFleetInventory, CurrentFleetInventoryError, CurrentFleetRegistry,
    CurrentFleetResolution, CurrentFleetTopology, read_current_fleet_inventory,
    resolve_current_fleet,
};
pub use model::{FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureReport};
#[doc(hidden)]
pub use ops::current_protocol::{
    CompiledCurrentComponentProvisioning, CompiledCurrentProtocolStep,
    CompiledCurrentRegistrySequence, CompiledCurrentStoreSequence, CurrentComponentGroupPlacement,
    CurrentRegistryStage, compile_current_component_provisioning,
    compile_current_protocol_sequence, compile_current_registry_sequence,
    compile_current_registry_sequence_with_status, compile_current_store_sequence_from_union,
};
pub use ops::{EnsurePaths, IcpEnsurePlatform, IcpEnsurePlatformError};
pub use workflow::{EnsureWorkflowError, apply, plan};
