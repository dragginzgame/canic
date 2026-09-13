//! Bind fresh local identities and reuse the maintained release initializer owner.

use crate::{
    durable_io,
    fleet_ensure::{
        model::{DesiredCanisterKind, DesiredFleet, DesiredPresence},
        ops as ensure_ops,
    },
    local_fleet::{LocalFleetError, model::*, view::LocalRootInstallationView},
};
use candid::Principal;
use canic_core::{cdk::utils::hash::sha256_hex, ids::CanonicalNetworkId};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn failure(error: impl std::fmt::Display) -> LocalFleetError {
    LocalFleetError::Preparation(error.to_string())
}

/// Admit only a fresh, exact local two-Root source before allocating any canister.
pub fn begin(
    directory: &Path,
    workspace: &Path,
    record: &LocalFleetRecord,
    source: &DesiredFleet,
) -> Result<LocalPreparationRecord, LocalFleetError> {
    require_workspace(directory, workspace)?;
    if source.environment != super::environment_name(record) {
        return Err(LocalFleetError::Identity);
    }
    let bootstrap = source
        .bootstrap
        .as_ref()
        .ok_or(LocalFleetError::Configuration)?;
    if record
        .release_build_id
        .is_some_and(|release| release != bootstrap.release_build_id)
    {
        return Err(LocalFleetError::Identity);
    }
    let root_key =
        canic_core::cdk::utils::hash::decode_hex(&record.root_key_der_hex).map_err(failure)?;
    let network = CanonicalNetworkId::from_der_root_trust_anchor(&root_key).map_err(failure)?;
    if !bootstrap.fresh_estate
        || bootstrap.canonical_network_id != network
        || source.protocol.is_none()
    {
        return Err(LocalFleetError::Identity);
    }
    let subnets = bootstrap
        .roots
        .iter()
        .map(|root| root.placement_subnet)
        .collect::<BTreeSet<_>>();
    if subnets.len() < 2
        || source.canisters.len() > usize::from(record.configuration.maximum_canisters)
    {
        return Err(LocalFleetError::Capacity);
    }
    if source.canisters.iter().any(|canister| {
        canister.principal.is_some()
            || canister.presence != DesiredPresence::Present
            || canister.replace
    }) {
        return Err(LocalFleetError::Identity);
    }
    for canister in &source.canisters {
        allocation_input(record, source, &canister.name)?;
    }
    for existing in &record.allocations {
        if allocation_input(record, source, &existing.input.name)? != existing.input {
            return Err(LocalFleetError::Identity);
        }
    }
    let release =
        crate::release_build::load_finalized_release_build(workspace, bootstrap.release_build_id)
            .map_err(failure)?;
    if release.record.build_network != canic_core::ids::BuildNetwork::Local {
        return Err(LocalFleetError::Identity);
    }
    let source_sha256 = sha256_hex(&serde_json::to_vec(source)?);
    let workspace_root = workspace.canonicalize()?;
    let path = directory.join("preparation.json");
    match durable_io::read_regular_bytes(&path, 4 * 1024 * 1024) {
        Ok(bytes) => {
            let retained: LocalPreparationRecord = serde_json::from_slice(&bytes)?;
            if retained.schema_version != 1
                || retained.source_sha256 != source_sha256
                || retained.workspace_root != workspace_root
            {
                return Err(LocalFleetError::Identity);
            }
            Ok(retained)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let prepared = LocalPreparationRecord {
                schema_version: 1,
                source_sha256,
                workspace_root,
                prepared: None,
                roots: Vec::new(),
                complete: false,
            };
            save(directory, &prepared)?;
            Ok(prepared)
        }
        Err(error) => Err(error.into()),
    }
}

fn save(directory: &Path, record: &LocalPreparationRecord) -> Result<(), LocalFleetError> {
    let bytes = serde_json::to_vec_pretty(record)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(LocalFleetError::Capacity);
    }
    durable_io::write_bytes(&directory.join("preparation.json"), &bytes)?;
    Ok(())
}

/// Convert each named source role into one explicit simulator allocation input.
pub fn allocation_input(
    record: &LocalFleetRecord,
    source: &DesiredFleet,
    name: &str,
) -> Result<LocalAllocationInput, LocalFleetError> {
    let canister = source
        .canisters
        .iter()
        .find(|canister| canister.name == name)
        .ok_or(LocalFleetError::Identity)?;
    let subnet = Principal::from_text(&canister.subnet).map_err(failure)?;
    let index = record
        .application_subnets
        .iter()
        .position(|known| *known == subnet)
        .ok_or(LocalFleetError::Identity)?;
    let role = match canister.kind {
        DesiredCanisterKind::Coordinator => "fleet_coordinator",
        DesiredCanisterKind::Root => "root",
        DesiredCanisterKind::Store => "wasm_store",
        DesiredCanisterKind::Pool => "pool",
        _ => return Err(LocalFleetError::Configuration),
    };
    Ok(LocalAllocationInput {
        name: name.into(),
        role: role.into(),
        application_subnet: u8::try_from(index).map_err(failure)?,
        controller: Principal::from_text(&source.operator).map_err(failure)?,
    })
}

/// Bind source logical identities to the complete local allocation set before initialization.
pub fn bind(
    directory: &Path,
    record: &LocalFleetRecord,
    source: &DesiredFleet,
    preparation: &LocalPreparationRecord,
) -> Result<LocalPreparationRecord, LocalFleetError> {
    let mut desired = source.clone();
    let principals = record
        .allocations
        .iter()
        .map(|entry| {
            Ok((
                entry.input.name.clone(),
                entry
                    .canister_id
                    .ok_or(LocalFleetError::Identity)?
                    .to_text(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, LocalFleetError>>()?;
    if principals.len() != source.canisters.len() {
        return Err(LocalFleetError::Identity);
    }
    for canister in &mut desired.canisters {
        canister.principal = Some(
            principals
                .get(&canister.name)
                .ok_or(LocalFleetError::Identity)?
                .clone(),
        );
        for name in &canister.controller_canisters {
            canister.controllers.push(
                principals
                    .get(name)
                    .ok_or(LocalFleetError::Identity)?
                    .clone(),
            );
        }
        canister.controller_canisters.clear();
        canister.controllers.sort();
        canister.controllers.dedup();
    }
    if preparation
        .prepared
        .as_ref()
        .is_some_and(|previous| previous != &desired)
    {
        return Err(LocalFleetError::Identity);
    }
    let mut next = preparation.clone();
    next.prepared = Some(desired);
    save(directory, &next)?;
    Ok(next)
}

/// Compile exact Root initialization through the same release authority used by Fleet Ensure.
pub fn root_installations(
    preparation: &LocalPreparationRecord,
) -> Result<Vec<LocalRootInstallationView>, LocalFleetError> {
    let desired = preparation
        .prepared
        .as_ref()
        .ok_or(LocalFleetError::Identity)?;
    let (_, desired_sha256) = desired_document(desired)?;
    let operation_id = crate::fleet_ensure::policy::operation_id(
        &desired_sha256,
        &desired.environment,
        &desired.fleet,
    );
    let principals = desired
        .canisters
        .iter()
        .map(|canister| {
            Ok((
                canister.name.clone(),
                canister
                    .principal
                    .clone()
                    .ok_or(LocalFleetError::Identity)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, LocalFleetError>>()?;
    let authorities =
        ensure_ops::compile_root_authorities(&preparation.workspace_root, desired, &principals)
            .map_err(failure)?;
    authorities
        .into_iter()
        .map(|(name, authority)| {
            let configured = desired
                .canisters
                .iter()
                .find(|canister| canister.name == name)
                .ok_or(LocalFleetError::Identity)?;
            let path = configured.wasm.as_ref().ok_or(LocalFleetError::Identity)?;
            let wasm = durable_io::read_regular_bytes(
                &preparation.workspace_root.join(path),
                32 * 1024 * 1024,
            )?;
            let wasm_sha256 = sha256_hex(&wasm);
            let arguments = ensure_ops::compile_arguments(&ensure_ops::CanicInitRequest {
                desired,
                init: configured
                    .canic_init
                    .as_ref()
                    .ok_or(LocalFleetError::Identity)?,
                operation_id: &operation_id,
                principals: &principals,
                root: &preparation.workspace_root,
                wasm: path,
                wasm_sha256: &wasm_sha256,
            })
            .map_err(failure)?;
            Ok(LocalRootInstallationView {
                canister_id: Principal::from_text(
                    principals.get(&name).ok_or(LocalFleetError::Identity)?,
                )
                .map_err(failure)?,
                name,
                operator: Principal::from_text(&desired.operator).map_err(failure)?,
                wasm,
                wasm_sha256,
                arguments_sha256: sha256_hex(&arguments),
                arguments,
                authority,
            })
        })
        .collect()
}

/// Serialize one current desired document; its byte hash also binds the existing Ensure operation ID.
pub fn desired_document(desired: &DesiredFleet) -> Result<(Vec<u8>, String), LocalFleetError> {
    let text = toml::to_string_pretty(desired).map_err(failure)?;
    let bytes = text.into_bytes();
    let digest = sha256_hex(&bytes);
    Ok((bytes, digest))
}

/// Reserve a bounded first-install attempt, or retain an already verified replay.
pub fn install_intent(
    directory: &Path,
    preparation: &LocalPreparationRecord,
    target: &LocalRootInstallationView,
    install_needed: bool,
) -> Result<LocalPreparationRecord, LocalFleetError> {
    let mut next = preparation.clone();
    let maximum = next
        .prepared
        .as_ref()
        .ok_or(LocalFleetError::Identity)?
        .maximum_stalled_observations;
    if let Some(entry) = next
        .roots
        .iter_mut()
        .find(|entry| entry.name == target.name)
    {
        if entry.wasm_sha256 != target.wasm_sha256
            || entry.arguments_sha256 != target.arguments_sha256
        {
            return Err(LocalFleetError::Identity);
        }
        if entry.verified && install_needed {
            return Err(LocalFleetError::Identity);
        }
        if install_needed {
            if entry.attempts >= maximum {
                return Err(LocalFleetError::Capacity);
            }
            entry.attempts += 1;
        }
    } else {
        if !install_needed {
            return Err(LocalFleetError::Identity);
        }
        if maximum == 0 {
            return Err(LocalFleetError::Capacity);
        }
        next.roots.push(LocalRootInstallRecord {
            name: target.name.clone(),
            wasm_sha256: target.wasm_sha256.clone(),
            arguments_sha256: target.arguments_sha256.clone(),
            attempts: 1,
            verified: false,
        });
    }
    save(directory, &next)?;
    Ok(next)
}

/// Mark only a Root whose installed authority was read and matched after submission.
pub fn verified_root(
    directory: &Path,
    preparation: &LocalPreparationRecord,
    name: &str,
) -> Result<LocalPreparationRecord, LocalFleetError> {
    let mut next = preparation.clone();
    next.roots
        .iter_mut()
        .find(|entry| entry.name == name)
        .ok_or(LocalFleetError::Identity)?
        .verified = true;
    save(directory, &next)?;
    Ok(next)
}

/// Retain the prepared desired bytes and freeze this local instance to their exact release.
pub fn finish(
    directory: &Path,
    record: &LocalFleetRecord,
    preparation: &LocalPreparationRecord,
) -> Result<LocalFleetRecord, LocalFleetError> {
    let desired = preparation
        .prepared
        .as_ref()
        .ok_or(LocalFleetError::Identity)?;
    let release = desired
        .bootstrap
        .as_ref()
        .ok_or(LocalFleetError::Identity)?
        .release_build_id;
    if record
        .release_build_id
        .is_some_and(|prior| prior != release)
    {
        return Err(LocalFleetError::Identity);
    }
    if preparation.roots.iter().any(|root| !root.verified) {
        return Err(LocalFleetError::Identity);
    }
    let (bytes, _) = desired_document(desired)?;
    durable_io::write_bytes(&directory.join("desired.toml"), &bytes)?;
    let mut next_preparation = preparation.clone();
    next_preparation.complete = true;
    save(directory, &next_preparation)?;
    let mut next = record.clone();
    next.release_build_id = Some(release);
    super::write_record(directory, &next)?;
    Ok(next)
}

/// Resolve a declared controller change from the complete prepared identity map.
pub fn controllers(
    desired: &DesiredFleet,
    name: &str,
) -> Result<crate::local_fleet::view::LocalControllersView, LocalFleetError> {
    let canister = desired
        .canisters
        .iter()
        .find(|canister| canister.name == name)
        .ok_or(LocalFleetError::Identity)?;
    let mut controllers = canister
        .controllers
        .iter()
        .map(|text| Principal::from_text(text).map_err(failure))
        .collect::<Result<Vec<_>, _>>()?;
    controllers.sort();
    Ok(crate::local_fleet::view::LocalControllersView {
        canister_id: Principal::from_text(
            canister
                .principal
                .as_deref()
                .ok_or(LocalFleetError::Identity)?,
        )
        .map_err(failure)?,
        operator: Principal::from_text(&desired.operator).map_err(failure)?,
        controllers,
    })
}

/// Return the exact prepared input through Fleet Ensure's maintained document contract.
pub fn loaded(
    preparation: &LocalPreparationRecord,
) -> Result<crate::fleet_ensure::LoadedDesiredFleet, LocalFleetError> {
    let desired = preparation
        .prepared
        .clone()
        .ok_or(LocalFleetError::Identity)?;
    let (_, sha256) = desired_document(&desired)?;
    Ok(crate::fleet_ensure::LoadedDesiredFleet { desired, sha256 })
}

/// Enroll the observed local trust through the existing network owner.
pub fn enroll(
    directory: &Path,
    workspace: &Path,
    environment: &str,
    record: &LocalFleetRecord,
) -> Result<(), LocalFleetError> {
    require_workspace(directory, workspace)?;
    if environment != super::environment_name(record) {
        return Err(LocalFleetError::Identity);
    }
    let key =
        canic_core::cdk::utils::hash::decode_hex(&record.root_key_der_hex).map_err(failure)?;
    let path = directory.join("root-key.der");
    durable_io::write_bytes(&path, &key)?;
    crate::network::enroll_network(crate::network::NetworkEnrollmentOptions {
        workspace_root: workspace,
        environment,
        root_key: &path,
        fingerprint: &sha256_hex(&key),
    })
    .map_err(failure)?;
    Ok(())
}

/// Keep the deployment workspace bound to the same owner that holds this local session lock.
pub fn require_workspace(directory: &Path, workspace: &Path) -> Result<(), LocalFleetError> {
    if directory.ancestors().nth(3) != Some(workspace.canonicalize()?.as_path()) {
        return Err(LocalFleetError::Identity);
    }
    Ok(())
}
