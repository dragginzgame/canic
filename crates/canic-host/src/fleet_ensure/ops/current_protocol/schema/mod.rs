//! Module: fleet_ensure::ops::current_protocol::schema
//!
//! Responsibility: admit each paid native mutation against its retained schema.
//! Does not own: transport, journal transitions, retry or response reconciliation.
//! Boundary: exact method identity, update mode and codec budgets precede effects.

use super::{CurrentFleetProtocolAction, CurrentProtocolError, protocol};
use canic_contracts::protocol::wire_declaration::MethodKind;

/// Verify the maintained native contract before its first or repeated effect.
pub(super) fn admit_mutation(
    action: &CurrentFleetProtocolAction,
    schema: &[u8],
) -> Result<(), CurrentProtocolError> {
    macro_rules! contract {
        ($method:path, $request:expr, $response:expr) => {{
            use $method as selected;
            (selected::METHOD, selected::METHOD_ID, $request, $response)
        }};
    }
    let (method, id, request, response) = match action {
        CurrentFleetProtocolAction::ObservePoolReadiness { .. } => return Ok(()),
        CurrentFleetProtocolAction::MaintainPoolReadiness { .. } => contract!(
            protocol::pool_control::maintain,
            protocol::pool_control::REQUEST_LIMITS,
            protocol::pool_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::ReconcilePoolAsset { .. } => contract!(
            protocol::pool_control::import,
            protocol::pool_control::REQUEST_LIMITS,
            protocol::pool_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::ActivateRegistry { .. } => contract!(
            protocol::registry_control::activate,
            protocol::registry_control::REQUEST_LIMITS,
            protocol::registry_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::JoinRoot { .. } => contract!(
            protocol::registry_control::join,
            protocol::registry_control::REQUEST_LIMITS,
            protocol::registry_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::ProvisionComponents { .. } => contract!(
            protocol::batch_control::prepare,
            protocol::batch_control::PLAN_LIMITS,
            protocol::batch_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::AdoptStore { .. } => contract!(
            protocol::root_store_control::adopt,
            protocol::root_store_control::REQUEST_LIMITS,
            protocol::root_store_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::BootstrapStore { .. } => contract!(
            protocol::root_store_control::bootstrap,
            protocol::root_store_control::REQUEST_LIMITS,
            protocol::root_store_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => contract!(
            protocol::component_control::prepare,
            protocol::component_control::REQUEST_LIMITS,
            protocol::component_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::SynchronizeRegistry { .. }
        | CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => contract!(
            protocol::registry_control::synchronize,
            protocol::registry_control::REQUEST_LIMITS,
            protocol::registry_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::PrepareStoreFixture { .. } => contract!(
            protocol::root_store_control::prepare,
            protocol::root_store_control::REQUEST_LIMITS,
            protocol::root_store_control::RESPONSE_LIMITS
        ),
        CurrentFleetProtocolAction::PublishStoreFixtureChunk { .. } => (
            protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
            protocol::fixture_bytes::upload::METHOD_ID,
            protocol::fixture_bytes::CHUNK_LIMITS,
            protocol::fixture_bytes::UPLOAD_RESPONSE_LIMITS,
        ),
        CurrentFleetProtocolAction::PublishStoreChunk { .. } => (
            protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
            protocol::template_bytes::METHOD_ID,
            protocol::template_bytes::REQUEST_LIMITS,
            protocol::template_bytes::RESPONSE_LIMITS,
        ),
    };
    crate::artifact_io::require_wire_method(
        schema,
        method,
        id,
        MethodKind::Update,
        request,
        response,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
