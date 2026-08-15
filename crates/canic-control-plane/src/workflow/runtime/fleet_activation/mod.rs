//! Module: workflow::runtime::fleet_activation
//!
//! Responsibility: coordinate root Fleet activation with root-owned Store and Component authority.
//! Does not own: endpoint authorization, Store inventory persistence, or activation state mutation.
//! Boundary: seals Component authority independently, then supplies only the Store
//! to the core Fleet activation workflow.

use super::template::WasmStorePublicationWorkflow;
use crate::workflow::{bootstrap::root as root_bootstrap, component_registry};
use canic_core::{
    control_plane_support::{
        error::InternalError, ops::runtime::ready::ReadyOps,
        view::fleet_activation::FleetActivationTransition,
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::fleet_activation::{
        FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
    },
};

/// Prepare the root and its exact Store infrastructure child for Fleet activation.
pub async fn prepare_root() -> Result<FleetActivationStatusResponse, InternalError> {
    root_bootstrap::bootstrap_init_root_canister().await;
    if !root_bootstrap::activation_preparation_complete() {
        return Err(InternalError::unavailable(
            "root bootstrap has not prepared the complete managed inventory; inspect bootstrap status and retry activation preparation",
        ));
    }
    let current = FleetActivationWorkflow::status()?;
    if current.phase == FleetActivationPhase::Active {
        component_registry::mark_root_runtime_activated(current.identity.operation_id)?;
        return Ok(current);
    }

    component_registry::seal_root_activation_inventory(current.identity.operation_id).await?;
    let wasm_store = WasmStorePublicationWorkflow::root_activation_wasm_store()?;
    FleetActivationWorkflow::prepare_root(wasm_store).await
}

/// Resume exact root Fleet activation and finish the independent Component inventory receipt.
pub async fn resume_root(
    request: FleetActivationResumeRequest,
) -> Result<FleetActivationTransition, InternalError> {
    let current = FleetActivationWorkflow::status()?;
    if current.phase == FleetActivationPhase::Prepared {
        component_registry::converge_root_activation_inventory(request.operation_id).await?;
    }
    let transition = FleetActivationWorkflow::resume_root(request).await?;
    if transition.status.phase != FleetActivationPhase::Active {
        return Err(InternalError::unavailable(
            "Fleet activation resume did not activate the root runtime",
        ));
    }
    component_registry::mark_root_runtime_activated(request.operation_id)?;
    root_bootstrap::bootstrap_init_root_canister().await;
    if !ReadyOps::is_ready() {
        return Err(InternalError::unavailable(
            "Fleet activation completed but root bootstrap is not ready; inspect bootstrap status and retry activation resume",
        ));
    }
    crate::workflow::canister_pool::start()?;
    Ok(transition)
}
