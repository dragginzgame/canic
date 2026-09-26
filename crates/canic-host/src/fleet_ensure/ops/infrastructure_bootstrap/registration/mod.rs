//! Build the exact Store/Registry successor from reviewed initialization and resolved IDs.

use crate::fleet_ensure::{
    model::{
        CanisterDisposition, CurrentFleetProtocolAction, EnsureAction, FleetEnsurePlan,
        FleetEnsurePlanScope, FleetEnsureStateRecord,
    },
    ops::{current_protocol, infrastructure_bootstrap::InfrastructureBootstrapError},
    policy::{expected_plan_sha256, successor_phase_burn},
};
use canic_core::dto::fleet_registry::FleetRegistry;
use std::path::Path;

/// No remote observations or effects occur while expanding the retained current protocol.
pub(in crate::fleet_ensure) fn compile(
    root: &Path,
    original: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    controlled_cycles: u128,
) -> Result<FleetEnsurePlan, InfrastructureBootstrapError> {
    let desired = original
        .reviewed_desired
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?
        .desired();
    let mut phase = original.clone();
    phase.infrastructure_bootstrap = None;
    phase.continuation = None;
    phase.recovery_review = None;
    phase.scope = FleetEnsurePlanScope::Full;
    for target in &mut phase.canisters {
        target.principal = state
            .pending_principals
            .get(&target.name)
            .or_else(|| state.principals.get(&target.name))
            .cloned()
            .or_else(|| target.principal.clone());
        if target.principal.is_none() {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        target.actions.clear();
        target.disposition = CanisterDisposition::Reuse;
    }
    phase.protocol_actions = current_protocol::compile_infrastructure_protocol(
        root,
        desired,
        state,
        &original.operation_id,
    )
    .map_err(|_| InfrastructureBootstrapError::Integrity)?;
    let burn = successor_phase_burn(desired, &phase)?;
    phase.conservation = crate::fleet_ensure::model::CycleConservation {
        estate_funding_domains: Vec::new(),
        expected_post_operation_cycles: controlled_cycles
            .checked_sub(burn)
            .ok_or(InfrastructureBootstrapError::Integrity)?,
        maximum_execution_burn_cycles: burn,
        maximum_new_funding_cycles: 0,
        maximum_operator_debit_cycles: 0,
        maximum_unavoidable_fee_cycles: 0,
        observed_controlled_cycles: controlled_cycles,
        retained_in_reused_canisters_cycles: controlled_cycles,
        scheduled_transfer_cycles: 0,
    };
    phase.plan_sha256 = expected_plan_sha256(&phase);
    Ok(phase)
}

/// The final activation action binds the exact Registry required by the import owner.
pub(in crate::fleet_ensure) fn registry(
    phase: &FleetEnsurePlan,
) -> Result<FleetRegistry, InfrastructureBootstrapError> {
    phase
        .protocol_actions
        .iter()
        .find_map(|action| {
            if let EnsureAction::FleetProtocol { action, .. } = action
                && let CurrentFleetProtocolAction::ActivateRegistry {
                    expected_registry, ..
                } = action.as_ref()
            {
                Some(expected_registry.clone())
            } else {
                None
            }
        })
        .ok_or(InfrastructureBootstrapError::Integrity)
}

/// Publish the verified active Registry locally while held pools remain outside tracked custody.
pub(in crate::fleet_ensure) fn bind_state(
    workspace: &Path,
    state: &mut FleetEnsureStateRecord,
    phase: &FleetEnsurePlan,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let desired = phase
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(invalid)?;
    let manifest = crate::release_set::load_persisted_canic_infrastructure_artifact_manifest(
        workspace,
        bootstrap.release_build_id,
    )
    .map_err(|_| invalid())?;
    for target in &phase.canisters {
        let configured = desired
            .canisters
            .iter()
            .find(|entry| entry.name == target.name)
            .ok_or_else(invalid)?;
        let artifact = manifest
            .manifest
            .entries
            .iter()
            .find(|entry| configured.wasm.as_ref() == Some(&entry.wasm_relative_path))
            .ok_or_else(invalid)?;
        let protocol = crate::protocol_binding::resolve_infrastructure_protocol_binding(
            workspace,
            &desired.environment,
            artifact,
        )
        .map_err(|_| invalid())?;
        let topology = state.topology.get_mut(&target.name).ok_or_else(invalid)?;
        if topology.module_hash.as_ref() != Some(&artifact.wasm_sha256_hex) {
            return Err(invalid());
        }
        topology.protocol_binding = Some(protocol.binding().clone());
        topology.role = Some(protocol.binding().role.to_string());
    }
    state.active_registry = Some(registry(phase)?);
    Ok(())
}
