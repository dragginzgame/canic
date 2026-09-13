//! Correlate Root operation detail before exposing host progress.

use crate::component_operation::{
    ComponentOperationError,
    model::{ComponentPhase, ComponentPlanRecord, ComponentProgressRecord},
    view::ComponentProgressObservation,
};
use canic_control_plane::dto::root::RootComponentOperationStatus;
use canic_core::dto::component_registry::{
    ComponentProvisioningOrigin, RootComponentAllocationPhase,
};

/// Project only a response bound to this exact operation, caller, Spec and release.
pub fn project_progress(
    plan: &ComponentPlanRecord,
    status: RootComponentOperationStatus,
) -> Result<ComponentProgressObservation, ComponentOperationError> {
    let allocation = status.allocation;
    let identity_matches = allocation.operation_id == plan.operation_id
        && allocation.component_spec == plan.authority.component_spec
        && allocation.spec_hash == plan.authority.spec_hash
        && allocation.role == plan.authority.role;
    let origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: plan.authority.operator,
    };
    if !identity_matches
        || allocation.release_set != plan.authority.release_set
        || allocation.provisioning_origin != origin
    {
        return Err(ComponentOperationError::Progress);
    }
    let phase = match allocation.phase {
        RootComponentAllocationPhase::Reserved => ComponentPhase::Reserved,
        RootComponentAllocationPhase::CreationIntent => ComponentPhase::CreationIntent,
        RootComponentAllocationPhase::Created => ComponentPhase::Created,
        RootComponentAllocationPhase::InstallIntent => ComponentPhase::InstallIntent,
        RootComponentAllocationPhase::Installed => ComponentPhase::Installed,
        RootComponentAllocationPhase::Verified => ComponentPhase::Verified,
        RootComponentAllocationPhase::Committed => ComponentPhase::Committed,
        RootComponentAllocationPhase::Removed => ComponentPhase::Removed,
    };
    Ok(ComponentProgressObservation {
        progress: Some(ComponentProgressRecord {
            allocation_sequence: allocation.allocation_sequence,
            component: allocation.component,
            phase,
            binding: allocation.installation.map(|evidence| evidence.binding),
            complete: status.complete,
        }),
    })
}
