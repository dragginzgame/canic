//! Module: fleet_ensure::ops::reinstall
//!
//! Responsibility: capture exact reset imports and immutable preparation inputs.
//! Does not own: reset admission, effect sequencing or retries.
//! Boundary: derived desired state is retained in the reviewed operation only.

pub(in crate::fleet_ensure) mod adoption;
pub(in crate::fleet_ensure) mod source;

use super::{EnsureStateError, FleetReinstallObservation};
use crate::fleet_ensure::model::{
    DesiredCanister, DesiredCanisterKind, DesiredFleet, DesiredPresence, FleetReinstallRecord,
    FleetReinstallSourceRecord, ReviewedDesiredFleetRecord,
};
use std::{collections::BTreeMap, path::Path};

/// Reconstruct the exact source seal to inspect before a reviewed authority credit.
pub(in crate::fleet_ensure) fn funding_seal(
    source: &FleetReinstallSourceRecord,
    binding: &crate::fleet_ensure::model::RootManagementBinding,
    kind: DesiredCanisterKind,
) -> Option<crate::fleet_ensure::model::EnsureAction> {
    let protocol = source.reviewed_desired.desired().protocol.as_ref()?;
    let candid = match kind {
        DesiredCanisterKind::Root => &protocol.root_candid,
        DesiredCanisterKind::Coordinator => &protocol.coordinator_candid,
        _ => return None,
    };
    Some(crate::fleet_ensure::model::EnsureAction::SealAuthority {
        authority_kind: kind,
        candid: candid.clone(),
        candid_sha256: source.candid_sha256_by_path.get(candid)?.clone(),
        name: binding.name.clone(),
        principal: binding.principal.clone(),
    })
}

/// Bind every resolved target artifact, including generated continuation contracts.
pub(in crate::fleet_ensure) fn target_artifacts_sha256(
    root: &Path,
    desired: &DesiredFleet,
) -> Result<String, EnsureStateError> {
    let artifacts = super::resolve_desired_artifacts(root, desired)?;
    let steps = artifacts
        .protocol_by_step
        .iter()
        .map(|(name, step)| {
            (
                name,
                &step.candid_sha256,
                &step.command_args_sha256,
                &step.expected_status_sha256,
                &step.status_args_sha256,
            )
        })
        .collect::<Vec<_>>();
    let continuation = artifacts.continuation.as_ref().map(|authority| {
        (
            &authority.app_config_sha256,
            &authority.application_artifact_union_sha256,
            &authority.coordinator_candid_sha256,
            &authority.root_candid_sha256,
            &authority.store_candid_sha256,
        )
    });
    let bytes = crate::fleet_ensure::json::to_vec(&(
        continuation,
        artifacts.drain_candid_sha256_by_canister,
        artifacts.init_arg_sha256_by_canister,
        artifacts.init_candid_sha256_by_canister,
        artifacts.wasm_sha256_by_canister,
        steps,
    ))
    .map_err(|source| EnsureStateError::Decode {
        path: root.to_path_buf(),
        source,
    })?;
    Ok(canic_core::cdk::utils::hash::sha256_hex(&bytes))
}

/// Bind the completed source input to its local infrastructure and protocol bytes.
pub(in crate::fleet_ensure) fn capture_source(
    root: &Path,
    desired: &DesiredFleet,
) -> Result<FleetReinstallSourceRecord, EnsureStateError> {
    let mut wasm_sha256_by_canister = BTreeMap::new();
    for canister in &desired.canisters {
        if let Some(path) = &canister.wasm {
            wasm_sha256_by_canister
                .insert(canister.name.clone(), super::artifact_sha256(root, path)?);
        }
    }
    Ok(FleetReinstallSourceRecord {
        reviewed_desired: ReviewedDesiredFleetRecord::capture(desired),
        wasm_sha256_by_canister,
        candid_sha256_by_path: candid_hashes(root, desired)?,
    })
}

pub(in crate::fleet_ensure) fn candid_hashes(
    root: &Path,
    desired: &DesiredFleet,
) -> Result<BTreeMap<String, String>, EnsureStateError> {
    let mut hashes = BTreeMap::new();
    if let Some(protocol) = &desired.protocol {
        for path in [
            &protocol.coordinator_candid,
            &protocol.root_candid,
            &protocol.store_candid,
        ] {
            hashes.insert(path.clone(), super::artifact_sha256(root, path)?);
        }
    }
    Ok(hashes)
}

pub(in crate::fleet_ensure) fn capture_imports(
    desired: &DesiredFleet,
    inventory: &FleetReinstallObservation,
) -> Option<DesiredFleet> {
    let mut retained = desired.clone();
    for canister in &mut retained.canisters {
        if let Some(authority) = inventory.authorities.get(&canister.name) {
            canister.principal = Some(authority.live.principal.clone());
        } else {
            canister.principal = Some(
                inventory
                    .observation
                    .canisters
                    .get(&canister.name)?
                    .as_ref()?
                    .principal
                    .clone(),
            );
        }
    }
    let bootstrap = retained.bootstrap.as_mut()?;
    bootstrap.fresh_estate = false;
    for root in &mut bootstrap.roots {
        root.canister_pool_imports.clear();
        for asset in inventory
            .assets
            .iter()
            .filter(|asset| asset.root == root.root)
        {
            let name = if let Some(configured) = retained
                .canisters
                .iter()
                .find(|c| c.principal.as_deref() == Some(&asset.principal))
            {
                if configured.kind != DesiredCanisterKind::Pool {
                    return None;
                }
                configured.name.clone()
            } else {
                let name = format!("reinstall-pool-{}", asset.principal);
                if retained.canisters.iter().any(|c| c.name == name) {
                    return None;
                }
                let cycles = root.limits.canister_pool.canister_cycles.to_string();
                retained.canisters.push(DesiredCanister {
                    canic_init: None,
                    controller_canisters: vec![root.root.clone()],
                    controllers: Vec::new(),
                    drain: None,
                    initial_cycles: cycles.clone(),
                    init_arg: None,
                    init_candid: None,
                    kind: DesiredCanisterKind::Pool,
                    minimum_cycles: cycles,
                    name: name.clone(),
                    parent: Some(root.root.clone()),
                    presence: DesiredPresence::Present,
                    principal: Some(asset.principal.clone()),
                    protocol_binding: None,
                    replace: false,
                    subnet: asset.subnet.clone(),
                    wasm: None,
                });
                name
            };
            root.canister_pool_imports.push(name);
        }
        root.canister_pool_imports.sort();
    }
    Some(retained)
}

pub(in crate::fleet_ensure) fn capture_intent(
    preparation: &FleetReinstallRecord,
    inventory: &FleetReinstallObservation,
) -> FleetReinstallRecord {
    let mut intent = preparation.clone();
    intent.assets.clone_from(&inventory.assets);
    intent
}

/// Retain the newly observed infrastructure and pool bindings after the reviewed Root reset.
pub(in crate::fleet_ensure) fn capture_after_activation_reset(
    intent: &FleetReinstallRecord,
    inventory: &FleetReinstallObservation,
) -> Option<FleetReinstallRecord> {
    let mut current = intent.clone();
    current.authorities = inventory
        .authorities
        .values()
        .map(authority_binding)
        .collect::<Option<Vec<_>>>()?;
    current.assets.clone_from(&inventory.assets);
    Some(current)
}

/// Capture the exact installed management authority used before each reset effect.
pub(in crate::fleet_ensure) fn authority_binding(
    observed: &crate::fleet_ensure::model::RootManagementCanisterObservation,
) -> Option<crate::fleet_ensure::model::RootManagementBinding> {
    let mut controllers = observed.live.controllers.clone();
    controllers.sort();
    Some(crate::fleet_ensure::model::RootManagementBinding {
        controllers,
        module_sha256: observed.live.module_sha256.clone()?,
        name: observed.name.clone(),
        principal: observed.live.principal.clone(),
        subnet: observed.subnet.clone(),
    })
}

/// Project the exact module installed by this review for terminal authority checks.
pub(in crate::fleet_ensure) fn terminal_authority(
    plan: &crate::fleet_ensure::model::FleetEnsurePlan,
    binding: &crate::fleet_ensure::model::RootManagementBinding,
) -> crate::fleet_ensure::model::RootManagementBinding {
    let mut expected = binding.clone();
    for action in plan.canisters.iter().flat_map(|canister| &canister.actions) {
        if let crate::fleet_ensure::model::EnsureAction::Install {
            name, wasm_sha256, ..
        } = action
            && name == &binding.name
        {
            expected.module_sha256.clone_from(wasm_sha256);
        }
    }
    expected
}
