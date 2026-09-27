//! Module: fleet_ensure::ops::reinstall::terminal::inventory
//!
//! Responsibility: cross-check completed source membership before live estate inspection.
//! Does not own: live custody, source execution, current authority or destructive admission.
//! Boundary: historical parentage and modules remain evidence, never fresh execution state.

pub(in crate::fleet_ensure) mod custody;
pub(in crate::fleet_ensure) mod ledger;
pub(in crate::fleet_ensure) mod membership;
pub(in crate::fleet_ensure) mod protocols;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        DesiredCanister, DesiredCanisterKind, DesiredFleetBootstrapRoot, DesiredPresence,
        MAX_FLEET_ENSURE_CANISTERS,
    },
    ops::{EnsurePaths, EnsureStateError, is_sha256},
    view::terminal_source::{
        CompletedReceiptAuditView,
        inventory::{
            CompletedCanisterInventoryView, CompletedEstateInventoryView,
            evidence::{
                CompletedStateEvidence, InventoryBootstrapEvidence, InventoryDeclarationEvidence,
                RegistryInventoryEvidence,
            },
        },
    },
};
use candid::Principal;
use canic_core::{
    dto::fleet_registry::FleetSubnetRootStatus,
    ids::{FleetBinding, FleetCoordinatorBinding, FleetKey, SubnetId},
};
use std::collections::{BTreeMap, BTreeSet};

/// Rebind the audited bytes before projecting their recorded inventory.
pub(in crate::fleet_ensure) fn inspect(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<CompletedEstateInventoryView, EnsureStateError> {
    let receipts = super::receipt_audit::inspect(paths, environment, fleet)?;
    let documents = super::documents::read(paths, environment, fleet)?;
    if receipts.documents != documents.bindings {
        return Err(invalid());
    }
    let desired = documents
        .plan
        .pointer("/reviewed_desired/desired")
        .cloned()
        .ok_or_else(invalid)?;
    project(receipts, decode(desired)?, decode(documents.state)?)
}

fn project(
    receipts: CompletedReceiptAuditView,
    desired: InventoryDeclarationEvidence,
    state: CompletedStateEvidence,
) -> Result<CompletedEstateInventoryView, EnsureStateError> {
    let bootstrap = &desired.bootstrap;
    let fleet = FleetBinding {
        app: bootstrap.app.clone(),
        fleet: FleetKey {
            canonical_network_id: bootstrap.canonical_network_id,
            fleet_id: bootstrap.fleet_id,
        },
    };
    let binding = &state.active_registry.authority.binding;
    let coordinator = principal(
        state
            .principals
            .get(&bootstrap.coordinator)
            .ok_or_else(invalid)?,
    )?;
    let expected_binding = FleetCoordinatorBinding {
        fleet: fleet.clone(),
        coordinator,
        coordinator_subnet: bootstrap.coordinator_subnet,
        recovery_controllers: bootstrap.recovery_controllers.clone(),
    };
    let declared_count = desired.canisters.len();
    let membership_counts_match = [
        state.principals.len(),
        state.topology.len(),
        state.retained_cycles_by_principal.len(),
    ]
    .into_iter()
    .all(|count| count == declared_count);
    if state.schema_version != 1
        || state.fleet.is_empty()
        || !state.pending_principals.is_empty()
        || binding != &expected_binding
        || !(1..=MAX_FLEET_ENSURE_CANISTERS).contains(&declared_count)
        || !membership_counts_match
    {
        return Err(invalid());
    }
    validate_reinstall_evidence(&state)?;
    let mut canisters = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut total = 0u128;
    for declared in &desired.canisters {
        let entry = project_canister(declared, &state)?;
        total = total
            .checked_add(entry.recorded_cycles)
            .ok_or_else(invalid)?;
        if !ids.insert(entry.principal) || canisters.insert(declared.name.clone(), entry).is_some()
        {
            return Err(invalid());
        }
    }
    let coordinator_entry = canisters.get(&bootstrap.coordinator).ok_or_else(invalid)?;
    if coordinator_entry.kind != DesiredCanisterKind::Coordinator
        || coordinator_entry.parent.is_some()
        || coordinator_entry.subnet != bootstrap.coordinator_subnet
    {
        return Err(invalid());
    }
    let mut assigned = BTreeSet::from([bootstrap.coordinator.clone()]);
    if bootstrap.roots.is_empty()
        || bootstrap.roots.len() != state.active_registry.fleet_subnet_roots.len()
        || bootstrap.roots.len() != receipts.initial_estate_funding_cycles_by_root.len()
    {
        return Err(invalid());
    }
    for root in &bootstrap.roots {
        validate_import_declarations(root, &desired.canisters)?;
        bind_root(
            root,
            bootstrap,
            &state.active_registry,
            &mut assigned,
            &mut canisters,
        )?;
        if !receipts
            .initial_estate_funding_cycles_by_root
            .contains_key(&root.root)
        {
            return Err(invalid());
        }
    }
    if assigned.len() != canisters.len() {
        return Err(invalid());
    }
    validate_parentage(&canisters)?;
    Ok(CompletedEstateInventoryView {
        coordinator_registry: project_coordinator(state.active_registry),
        receipts,
        fleet,
        release_build_id: bootstrap.release_build_id,
        coordinator,
        canisters,
        recorded_controlled_canister_cycles: total,
    })
}

fn project_coordinator(
    registry: RegistryInventoryEvidence,
) -> crate::fleet_ensure::CompletedCoordinatorMembershipView {
    let binding = registry.authority.binding;
    crate::fleet_ensure::CompletedCoordinatorMembershipView {
        fleet: binding.fleet,
        coordinator: binding.coordinator,
        coordinator_subnet: binding.coordinator_subnet,
        epoch: registry.authority.epoch,
        revision: registry.revision,
        roots: registry.fleet_subnet_roots,
    }
}

fn project_canister(
    declared: &DesiredCanister,
    state: &CompletedStateEvidence,
) -> Result<CompletedCanisterInventoryView, EnsureStateError> {
    let id = declared.principal.as_deref().ok_or_else(invalid)?;
    let id = principal(id)?;
    let topology = state.topology.get(&declared.name).ok_or_else(invalid)?;
    let principal_matches = state.principals.get(&declared.name) == declared.principal.as_ref();
    let allocated_pool = declared.kind == DesiredCanisterKind::Pool
        && topology.kind == DesiredCanisterKind::Component;
    if declared.presence != DesiredPresence::Present
        || declared.replace
        || declared.drain.is_some()
        || !principal_matches
        || (declared.kind != topology.kind && !allocated_pool)
        || (!allocated_pool && declared.parent != topology.parent)
    {
        return Err(invalid());
    }
    match (
        &topology.module_hash,
        &topology.protocol_binding,
        &topology.role,
    ) {
        (None, None, Some(role))
            if topology.kind == DesiredCanisterKind::Pool && role == "canister_pool_asset" => {}
        (Some(hash), Some(binding), Some(role))
            if topology.kind != DesiredCanisterKind::Pool
                && is_sha256(hash)
                && binding.role.to_string() == *role
                && !binding.release_identity.is_empty() => {}
        _ => return Err(invalid()),
    }
    let mut controllers = BTreeSet::new();
    for controller in &declared.controllers {
        if !controllers.insert(principal(controller)?) {
            return Err(invalid());
        }
    }
    for name in &declared.controller_canisters {
        let controller = state.principals.get(name).ok_or_else(invalid)?;
        if !controllers.insert(principal(controller)?) {
            return Err(invalid());
        }
    }
    if controllers.is_empty() {
        return Err(invalid());
    }
    Ok(CompletedCanisterInventoryView {
        principal: id,
        subnet: SubnetId::from_principal(principal(&declared.subnet)?),
        kind: topology.kind,
        parent: topology.parent.clone(),
        root: None,
        module_sha256: topology.module_hash.clone(),
        protocol_binding: topology.protocol_binding.clone(),
        originally_declared_controllers: controllers.into_iter().collect(),
        recorded_cycles: *state
            .retained_cycles_by_principal
            .get(&id.to_text())
            .ok_or_else(invalid)?,
    })
}

fn bind_root(
    root: &DesiredFleetBootstrapRoot,
    bootstrap: &InventoryBootstrapEvidence,
    registry: &RegistryInventoryEvidence,
    assigned: &mut BTreeSet<String>,
    entries: &mut BTreeMap<String, CompletedCanisterInventoryView>,
) -> Result<(), EnsureStateError> {
    let entry = entries.get(&root.root).ok_or_else(invalid)?;
    let mut rows = registry
        .fleet_subnet_roots
        .iter()
        .filter(|row| row.fleet_subnet_root == entry.principal);
    let row = rows.next().ok_or_else(invalid)?;
    let membership_matches = entry.kind == DesiredCanisterKind::Root
        && entry.subnet == root.placement_subnet
        && entry.parent.as_ref() == Some(&bootstrap.coordinator)
        && row.placement_subnet == root.placement_subnet
        && row.component_admissions == root.component_admissions
        && row.component_topology_digest == root.component_topology_digest
        && row.active_release_set.release_build_id == bootstrap.release_build_id
        && row.limits == root.limits
        && row.funding == root.funding
        && row.status == FleetSubnetRootStatus::Active;
    if !membership_matches
        || rows.next().is_some()
        || root.canister_pool_imports.len() > root.limits.canister_pool.maximum_size as usize
    {
        return Err(invalid());
    }
    for name in std::iter::once(&root.root)
        .chain(std::iter::once(&root.store))
        .chain(&root.canister_pool_imports)
    {
        if !assigned.insert(name.clone()) {
            return Err(invalid());
        }
        let entry = entries.get_mut(name).ok_or_else(invalid)?;
        if entry.subnet != root.placement_subnet {
            return Err(invalid());
        }
        let kind_matches = if name == &root.root {
            entry.kind == DesiredCanisterKind::Root
        } else if name == &root.store {
            entry.kind == DesiredCanisterKind::Store && entry.parent.as_ref() == Some(&root.root)
        } else {
            matches!(
                entry.kind,
                DesiredCanisterKind::Pool | DesiredCanisterKind::Component
            )
        };
        if !kind_matches {
            return Err(invalid());
        }
        entry.root = Some(root.root.clone());
    }
    Ok(())
}

fn validate_import_declarations(
    root: &DesiredFleetBootstrapRoot,
    declared: &[DesiredCanister],
) -> Result<(), EnsureStateError> {
    for name in &root.canister_pool_imports {
        let canister = declared
            .iter()
            .find(|canister| &canister.name == name)
            .ok_or_else(invalid)?;
        if canister.kind != DesiredCanisterKind::Pool
            || canister.parent.as_ref() != Some(&root.root)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Walk every application parent chain to its owning Root, rejecting cycles and cross-Root edges.
fn validate_parentage(
    entries: &BTreeMap<String, CompletedCanisterInventoryView>,
) -> Result<(), EnsureStateError> {
    for (name, entry) in entries {
        if !matches!(
            entry.kind,
            DesiredCanisterKind::Pool | DesiredCanisterKind::Component
        ) {
            continue;
        }
        let root = entry.root.as_ref().ok_or_else(invalid)?;
        let mut visited = BTreeSet::from([name.as_str()]);
        let mut child = entry;
        loop {
            let parent_name = child.parent.as_deref().ok_or_else(invalid)?;
            if !visited.insert(parent_name) {
                return Err(invalid());
            }
            let parent = entries.get(parent_name).ok_or_else(invalid)?;
            if parent.root.as_ref() != Some(root) || parent.subnet != entry.subnet {
                return Err(invalid());
            }
            if parent_name == root && parent.kind == DesiredCanisterKind::Root {
                break;
            }
            if child.kind == DesiredCanisterKind::Pool
                || parent.kind != DesiredCanisterKind::Component
            {
                return Err(invalid());
            }
            child = parent;
        }
    }
    Ok(())
}

fn validate_reinstall_evidence(state: &CompletedStateEvidence) -> Result<(), EnsureStateError> {
    if state.completed_reinstalls.len() != state.completed_reinstall_action_sha256.len() {
        return Err(invalid());
    }
    for name in state.completed_reinstalls.keys() {
        if !state.principals.contains_key(name)
            || !state
                .completed_reinstall_action_sha256
                .get(name)
                .is_some_and(|hash| is_sha256(hash))
        {
            return Err(invalid());
        }
    }
    match &state.completed_reinstall_operation_id {
        Some(operation) if is_sha256(operation) => Ok(()),
        None if state.completed_reinstalls.is_empty() => Ok(()),
        _ => Err(invalid()),
    }
}

fn principal(text: &str) -> Result<Principal, EnsureStateError> {
    let value = Principal::from_text(text).map_err(|_| invalid())?;
    if value == Principal::anonymous()
        || value == Principal::management_canister()
        || value.to_text() != text
    {
        return Err(invalid());
    }
    Ok(value)
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, EnsureStateError> {
    serde_json::from_value(value).map_err(|_| invalid())
}

const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidTerminalSource
}
