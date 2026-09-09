//! Module: fleet_ensure::ops::current_protocol::inactive_activation
//!
//! Responsibility: observe installed activation and initial-inventory receipts for reset review.
//! Does not own: reset admission, source supersession, payments or effect execution.
//! Boundary: only retained, hash-bound protected read contracts are used.

use crate::fleet_ensure::model::{FleetActivationSourceRecord, RootActivationResetRecord};
use canic_control_plane::dto::template::StoreOperationStatusResponse;
use canic_core::dto::{
    component_provisioning::RootComponentProvisioningPhase,
    fleet_activation::{FleetActivationIdentity, FleetActivationPhase},
};

use super::{
    ComponentRegistryAuthority, CurrentFleetProtocolAction, CurrentProtocolError, IcpCli,
    OperationStatusRequest, Path, Principal, ResolvedProtocolAction, RootOperationStatusResponse,
    RootStatusRequestFragment, RootStatusResponseFragment, StoreStatusRequest, StoreStatusResponse,
    canic_init, protocol, query_root_operation, query_with_candid, read_sha256,
};

/// Read the exact Prepared Root and its conflicting source-operation Store binding.
#[expect(
    clippy::too_many_lines,
    reason = "one source observation binds registry, provisioning and both installed activation owners"
)]
pub(in crate::fleet_ensure::ops) fn observe(
    icp: &IcpCli,
    workspace: &Path,
    source: &FleetActivationSourceRecord,
    root_name: &str,
    root: Principal,
) -> Result<RootActivationResetRecord, CurrentProtocolError> {
    let preparation = source.registry_preparations.iter().find(|action| {
        matches!(action, crate::fleet_ensure::model::EnsureAction::FleetProtocol { principal, .. } if principal == &root.to_text())
    }).ok_or(CurrentProtocolError::ResponseMismatch)?;
    let resolved = ResolvedProtocolAction::from_action(workspace, preparation)?;
    let CurrentFleetProtocolAction::PrepareComponentRegistry { expected, request } =
        resolved.action
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let RootStatusResponseFragment::ComponentRegistry(registry) = query_with_candid(
        icp,
        &resolved.candid_path,
        root,
        protocol::CANIC_ROOT_STATUS,
        &RootStatusRequestFragment::ComponentRegistry(request.clone()),
    )?
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    if ComponentRegistryAuthority::from(&registry) != ComponentRegistryAuthority::from(expected) {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let sealed = registry
        .initial_inventory
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let operation = ResolvedProtocolAction::from_action(workspace, &source.provisioning)?;
    let CurrentFleetProtocolAction::ProvisionComponents { request, plan_hash } = operation.action
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let batch = request
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == root)
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let Some(RootOperationStatusResponse::ProvisionComponents(provisioning)) =
        query_root_operation(icp, &resolved.candid_path, root, request.operation_id)?
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let inventory_complete = [
        registry.fleet_subnet_root == root,
        registry.reserved_component_instances == 0,
        registry
            .committed_component_instances
            .checked_add(registry.managed_descendants)
            == Some(registry.known_created_component_canisters),
        registry.next_allocation_sequence == u64::from(registry.committed_component_instances) + 1,
        sealed.component_count == registry.committed_component_instances,
        sealed.sealed_at_ns != 0,
        sealed.inventory_hash != [0; 32],
        !sealed.root_runtime_activated,
        provisioning.operation_id == request.operation_id,
        provisioning.plan_hash == *plan_hash,
        provisioning.fleet_subnet_root == root,
        provisioning.phase == RootComponentProvisioningPhase::Published,
        provisioning.component_count == sealed.component_count,
        provisioning.activated_component_count == sealed.component_count,
        !provisioning.root_runtime_active,
        provisioning.estate_funding_required.is_none(),
        provisioning.receipt_content_hash != [0; 32],
    ]
    .into_iter()
    .all(|fact| fact);
    if !inventory_complete {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let expected_root_operation = canic_init::install_id(&source.operation_id, "root", root_name);
    if sealed.fleet_activation_operation_id == expected_root_operation {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let Some(RootOperationStatusResponse::FleetActivation(activation)) = query_root_operation(
        icp,
        &resolved.candid_path,
        root,
        sealed.fleet_activation_operation_id,
    )?
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let expected_activation = FleetActivationIdentity {
        fleet: batch.root.authority.binding.fleet.clone(),
        operation_id: sealed.fleet_activation_operation_id,
        release_build_id: batch.active_release_set.release_build_id,
    };
    if activation.identity != expected_activation
        || activation.phase != FleetActivationPhase::Prepared
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let RootStatusResponseFragment::FleetAuthority(authority) = query_with_candid(
        icp,
        &resolved.candid_path,
        root,
        protocol::CANIC_ROOT_STATUS,
        &RootStatusRequestFragment::FleetAuthority,
    )?
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    if authority.binding != batch.root || authority.initial_release_set != batch.active_release_set
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let store = source
        .stores
        .iter()
        .find(|store| store.principal == authority.wasm_store_authority.wasm_store.to_text())
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let candid = workspace.join(&store.candid);
    if read_sha256(&candid)? != store.candid_sha256 {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let store_operation = canic_init::install_id(&source.operation_id, "store", root_name);
    let StoreStatusResponse::Operation(StoreOperationStatusResponse::FleetActivation(
        store_activation,
    )) = query_with_candid(
        icp,
        &candid,
        authority.wasm_store_authority.wasm_store,
        protocol::CANIC_WASM_STORE_STATUS,
        &StoreStatusRequest::Operation(OperationStatusRequest {
            operation_id: store_operation,
        }),
    )?
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let store_identity = FleetActivationIdentity {
        operation_id: store_operation,
        ..expected_activation
    };
    if store_activation.identity != store_identity
        || store_activation.phase != FleetActivationPhase::Prepared
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(RootActivationResetRecord {
        root: root_name.to_string(),
        activation_operation_id: sealed.fleet_activation_operation_id,
        inventory_hash: sealed.inventory_hash,
        provisioning_receipt_hash: provisioning.receipt_content_hash,
        component_count: sealed.component_count,
        managed_descendants: registry.managed_descendants,
    })
}
