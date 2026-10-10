//! Module: fleet_ensure::ops::current_protocol::fresh
//!
//! Responsibility: expand the protocol bound to reviewed fresh initialization.
//! Does not own: phase scheduling, spending admission, or remote effects.
//! Boundary: resolves exact retained Principals and current protocol authority.

use super::{
    CurrentProtocolError, bind_action, canic_init, compile_current_infrastructure_sequence,
    compile_current_protocol_sequence, compile_current_registry_sequence,
    compile_current_store_sequence, current_protocol_stage, desired_cycles, operation_bytes,
    retained_principal,
};
use crate::fleet_ensure::model::{
    CurrentFleetProtocolAction, DesiredFleet, EnsureAction, FleetEnsureStateRecord,
};
use candid::Principal;
use canic_contracts::dto::pool::PoolCanisterRequest;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    shared_support::fleet_admission_policy::bind_initial_fleet_admission_policy,
};
use std::{collections::BTreeMap, path::Path};

/// Expand only the protocol selected by the reviewed fresh installation inputs.

pub(in crate::fleet_ensure::ops) fn compile(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    operation_id: &str,
) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
    compile_selected(root, desired, state, operation_id, false)
}

/// Expand Store and Registry setup while supplied pool capacity remains fenced.
pub(in crate::fleet_ensure::ops) fn compile_infrastructure(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    operation_id: &str,
) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
    compile_selected(root, desired, state, operation_id, true)
}

fn compile_selected(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    operation_id: &str,
    infrastructure_only: bool,
) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let intent = desired
        .protocol
        .as_ref()
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let principals = resolved_principals(desired, state)?;
    let named_authorities = canic_init::compile_root_authorities(root, desired, &principals)?;
    let mut authorities = named_authorities
        .iter()
        .map(|(_, authority)| authority.clone())
        .collect::<Vec<_>>();
    authorities.sort_unstable_by_key(|authority| authority.binding.placement_subnet);
    let authority = authorities
        .first()
        .ok_or(CurrentProtocolError::ResponseMismatch)?
        .binding
        .authority
        .clone();
    let admission =
        bind_initial_fleet_admission_policy(authority.binding.fleet.clone(), &bootstrap.admission)
            .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let configuration = &bootstrap.component_deployment_configuration;
    let genesis = FleetRegistryOps::compile_genesis(
        &bootstrap.app,
        authority,
        &configuration.component_topology,
        admission,
    )
    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let registry = compile_current_registry_sequence(
        desired,
        state,
        &configuration.component_topology,
        &genesis,
        &authorities,
    )?;
    let identity = operation_bytes(operation_id)?;
    let stores = authorities
        .iter()
        .map(|authority| {
            compile_current_store_sequence(
                root,
                &configuration.component_topology,
                authority,
                identity,
            )
            .map(|sequence| (authority.binding.fleet_subnet_root, sequence))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut compiled = if infrastructure_only {
        compile_current_infrastructure_sequence(
            configuration,
            &registry,
            &authorities,
            &stores,
            identity,
        )?
    } else {
        compile_current_protocol_sequence(
            desired,
            state,
            configuration,
            &registry,
            &authorities,
            &stores,
            identity,
        )?
    };
    compiled.sort_by_key(|step| current_protocol_stage(&step.action));
    let burn = desired_cycles(
        "maximum_update_burn_cycles",
        &desired.maximum_update_burn_cycles,
    )?;
    let mut actions = if infrastructure_only {
        Vec::new()
    } else {
        compile_imports(root, desired, state, &principals, burn)?
    };
    for step in compiled {
        actions.push(bind_action(
            root,
            desired,
            state,
            intent,
            step.action,
            step.target,
            step.name,
            burn,
        )?);
    }
    Ok(actions)
}

fn resolved_principals(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
) -> Result<BTreeMap<String, String>, CurrentProtocolError> {
    desired
        .canisters
        .iter()
        .map(|canister| {
            retained_principal(desired, state, &canister.name)
                .map(|principal| (canister.name.clone(), principal))
                .ok_or(CurrentProtocolError::ResponseMismatch)
        })
        .collect()
}

fn compile_imports(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    principals: &BTreeMap<String, String>,
    burn: u128,
) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let intent = desired
        .protocol
        .as_ref()
        .ok_or(CurrentProtocolError::ResponseMismatch)?;
    let mut actions = Vec::new();
    for input in &bootstrap.roots {
        if input.capacity_import_bootstrap.is_some() {
            // These sources have not entered Root custody. The reviewed import owner
            // must hand them off and publish receipts before ordinary reconciliation.
            continue;
        }
        let target = Principal::from_text(
            principals
                .get(&input.root)
                .ok_or(CurrentProtocolError::ResponseMismatch)?,
        )
        .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
        for name in &input.canister_pool_imports {
            let principal = principals
                .get(name)
                .ok_or(CurrentProtocolError::ResponseMismatch)?;
            let canister_id = Principal::from_text(principal)
                .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
            actions.push(bind_action(
                root,
                desired,
                state,
                intent,
                CurrentFleetProtocolAction::ReconcilePoolAsset {
                    request: PoolCanisterRequest { canister_id },
                    minimum_cycles: input.limits.canister_pool.canister_cycles.clone(),
                },
                target,
                format!("pool-reconcile-{principal}"),
                burn,
            )?);
        }
    }
    Ok(actions)
}
