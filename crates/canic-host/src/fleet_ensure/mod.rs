//! Module: fleet_ensure
//!
//! Responsibility: expose the sole current-generation desired-state Fleet reconciler.
//! Does not own: historical install plans, migration, retained-repair compatibility, or runtime policy.
//! Boundary: current desired state plus live IC observation are the only planning authorities.

pub mod dto;
mod generate;
mod inventory;
mod json;
pub mod model;
pub mod ops;
pub mod policy;
pub mod view;
pub mod workflow;

#[cfg(test)]
mod tests;

pub use dto::{DesiredFleetLoadError, LoadedDesiredFleet, load_desired_fleet};
#[cfg(feature = "local-fleet")]
pub(crate) use generate::generate_local_fleet;

pub use generate::{
    FleetGenerateError, FleetGenerateRequest, FreshEstateSeedRequest, GeneratedDesiredFleet,
    capacity_import::{CapacityImportInventoryError, prepare_capacity_import_inventory},
    fresh_pool_creation_funding, generate_desired_fleet,
    infrastructure_bootstrap::generate_infrastructure_bootstrap,
    initialize_fresh_estate_seed,
    preflight::{FleetGenerationInputsRequest, validate_generation_inputs},
};
pub use inventory::{
    CurrentFleetDiscovery, CurrentFleetInventory, CurrentFleetInventoryError, CurrentFleetRegistry,
    CurrentFleetResolution, CurrentFleetSummary, CurrentFleetTopology, discover_current_fleets,
    read_last_converged_fleet_inventory, resolve_current_fleet,
};
#[doc(hidden)]
pub use json::report_json_value;
pub use model::{FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureReport};
#[doc(hidden)]
pub use ops::current_protocol::{
    CompiledCurrentComponentProvisioning, CompiledCurrentProtocolStep,
    CompiledCurrentRegistrySequence, CompiledCurrentStoreSequence, CurrentComponentGroupPlacement,
    CurrentRegistryStage, compile_current_component_provisioning,
    compile_current_infrastructure_sequence, compile_current_protocol_sequence,
    compile_current_registry_sequence, compile_current_registry_sequence_with_status,
    compile_current_store_sequence_from_union,
};
pub use ops::reinstall::terminal::inventory::protocols::CompletedSourceProtocolError;
pub use ops::{EnsurePaths, IcpEnsurePlatform, IcpEnsurePlatformError};
pub use view::terminal_source::CompletedReceiptAuditView;
pub use view::terminal_source::inventory::coordinator::CompletedCoordinatorMembershipView;
pub use view::terminal_source::inventory::ledger::{
    CompletedLedgerAccountView, CompletedLedgerBalancesView,
};
pub use view::terminal_source::inventory::membership::{
    CompletedEstateMembershipView, CompletedPoolAssetView, CompletedRootMembershipView,
    CompletedWorkloadAllocationView,
};
pub use view::terminal_source::inventory::{
    CompletedCanisterCustodyView, CompletedCanisterInventoryView, CompletedEstateCustodyView,
    CompletedEstateInventoryView, CompletedSourceInspectionView,
};
pub use workflow::{
    EnsureWorkflowError, apply, plan, plan_reinstall, retained_in_progress_plan,
    retained_reinstall_apply_plan,
};
