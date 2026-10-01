//! Bind installed infrastructure to the completed Fleet's current immutable artifact manifest.

use crate::{
    fleet_ensure::{
        model::{
            DesiredCanisterKind, DesiredFleet, DesiredPresence, FleetEnsureStateRecord,
            capacity_import::admission::{
                CapacityImportInfrastructureKind, CapacityImportInfrastructureRecord,
            },
        },
        ops::{
            EnsurePaths,
            capacity_import::{admission::declarations::hash, journal::CapacityImportJournalError},
            certified_custody,
        },
    },
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_set::{
        CanicInfrastructureRole, load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use candid::Principal;
use ic_agent::Agent;

pub(super) async fn inspect(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    agent: &Agent,
) -> Result<Vec<CapacityImportInfrastructureRecord>, CapacityImportJournalError> {
    let missing = || CapacityImportJournalError::InfrastructureRequired;
    let changed = || CapacityImportJournalError::InfrastructureChanged;
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(missing)?;
    let manifest = load_persisted_canic_infrastructure_artifact_manifest(
        &paths.workspace,
        bootstrap.release_build_id,
    )
    .map_err(|_| missing())?;
    let release =
        load_persisted_current_release_set_manifest(&paths.workspace, bootstrap.release_build_id)
            .map_err(|_| missing())?;
    if release.manifest.infrastructure_artifact_manifest_sha256 != manifest.digest {
        return Err(changed());
    }
    let mut result = Vec::new();
    for canister in &desired.canisters {
        let (kind, role) = match canister.kind {
            DesiredCanisterKind::Coordinator => (
                CapacityImportInfrastructureKind::Coordinator,
                CanicInfrastructureRole::FleetCoordinator,
            ),
            DesiredCanisterKind::Root => (
                CapacityImportInfrastructureKind::Root,
                CanicInfrastructureRole::FleetSubnetRoot,
            ),
            DesiredCanisterKind::Store => {
                let parent = canister.parent.as_ref().ok_or_else(missing)?;
                let root = Principal::from_text(state.principals.get(parent).ok_or_else(missing)?)
                    .map_err(|_| missing())?;
                (
                    CapacityImportInfrastructureKind::Store { root },
                    CanicInfrastructureRole::WasmStore,
                )
            }
            _ => continue,
        };
        if canister.presence != DesiredPresence::Present {
            return Err(missing());
        }
        let artifact = manifest
            .manifest
            .entries
            .iter()
            .find(|entry| entry.role == role)
            .ok_or_else(missing)?;
        let expected = hash(&artifact.wasm_sha256_hex)?;
        let retained = state.topology.get(&canister.name).ok_or_else(missing)?;
        let protocol = resolve_infrastructure_protocol_binding(
            &paths.workspace,
            &desired.environment,
            artifact,
        )
        .map_err(|_| missing())?;
        if artifact.protocol_release_identity != env!("CARGO_PKG_VERSION")
            || canister.wasm.as_ref() != Some(&artifact.wasm_relative_path)
            || retained.module_hash.as_ref() != Some(&artifact.wasm_sha256_hex)
            || retained.kind != canister.kind
            || retained.parent != canister.parent
            || retained.protocol_binding.as_ref() != Some(protocol.binding())
        {
            return Err(changed());
        }
        let principal =
            Principal::from_text(state.principals.get(&canister.name).ok_or_else(missing)?)
                .map_err(|_| missing())?;
        if canister
            .principal
            .as_ref()
            .is_some_and(|declared| *declared != principal.to_text())
        {
            return Err(changed());
        }
        let observed = certified_custody::observe_one(agent, principal)
            .await
            .map_err(|_| changed())?;
        if observed.module_sha256.as_deref().map(hash).transpose()? != Some(expected) {
            return Err(changed());
        }
        result.push(CapacityImportInfrastructureRecord {
            kind,
            principal,
            subnet: observed.subnet,
            controllers: observed.controllers,
            module_sha256: expected,
        });
    }
    Ok(result)
}
