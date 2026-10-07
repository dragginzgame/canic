//! Admit ordinary convergence only after setup and every held import have durable completion.

use crate::fleet_ensure::{
    model::{
        DesiredFleet, FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsurePlanScope, FleetEnsureStateRecord,
        capacity_import::{CapacityImportJournalRecord, operation::CapacityImportPublicationKind},
    },
    ops::{
        EnsurePaths,
        capacity_import::{journal, publication},
        infrastructure_bootstrap::{InfrastructureBootstrapError, terminal},
    },
};
use ic_host_fs::durable::create_new_bytes_with_parents;
use ic_host_fs::read::{read_file_no_follow, read_optional_file_no_follow};

use candid::Principal;
use std::{collections::BTreeSet, path::Path};

const MAX_BYTES: usize = 8 * 1024 * 1024;

/// The caller owns the Fleet lock. Archive original bytes before planning refreshes local state.
pub(in crate::fleet_ensure) fn prepare(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    desired: &DesiredFleet,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    if plan.scope != FleetEnsurePlanScope::InfrastructureBootstrap
        || journal.completion != FleetEnsureCompletion::Converged
    {
        return Err(invalid());
    }
    verify_desired(plan, state, desired)?;
    let archive = paths
        .plan
        .with_file_name("infrastructure-bootstrap-completed")
        .join(&plan.plan_sha256);
    let archived_state = archive.join("state.json");
    let original_state =
        match read_optional_file_no_follow(&archived_state, MAX_BYTES).map_err(|_| invalid())? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => state.clone(),
        };
    if state.principals != original_state.principals
        || state.pending_principals != original_state.pending_principals
        || state.topology != original_state.topology
        || state.active_registry != original_state.active_registry
        || terminal::read_receipt(paths, plan, journal, &original_state)?.is_none()
    {
        return Err(invalid());
    }
    verify_imports(paths, plan, &original_state)?;
    retain(
        &archive.join("plan.json"),
        &read_file_no_follow(&paths.plan, MAX_BYTES).map_err(std::io::Error::from)?,
    )?;
    retain(
        &archive.join("journal.json"),
        &read_file_no_follow(&paths.journal, MAX_BYTES).map_err(std::io::Error::from)?,
    )?;
    if !archived_state.try_exists()? {
        retain(
            &archived_state,
            &read_file_no_follow(&paths.state, MAX_BYTES).map_err(std::io::Error::from)?,
        )?;
    }
    Ok(())
}

fn retain(path: &Path, bytes: &[u8]) -> Result<(), InfrastructureBootstrapError> {
    match read_optional_file_no_follow(path, MAX_BYTES)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?
    {
        Some(existing) if existing == bytes => Ok(()),
        Some(_) => Err(InfrastructureBootstrapError::Integrity),
        None => Ok(create_new_bytes_with_parents(path, bytes)?),
    }
}

fn verify_imports(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(invalid)?;
    let mut records = Vec::new();
    read_import(
        &paths.plan.with_file_name("capacity-import.json"),
        &mut records,
    )?;
    let history = paths.plan.with_file_name("capacity-import-history");
    if history.try_exists()? {
        for (index, entry) in std::fs::read_dir(history)?.enumerate() {
            if index >= crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS {
                return Err(invalid());
            }
            read_import(&entry?.path(), &mut records)?;
        }
    }
    let source = plan.infrastructure_bootstrap.as_ref().ok_or_else(invalid)?;
    let registry = state.active_registry.as_ref().ok_or_else(invalid)?;
    if source.estate_seed.is_some()
        && !super::publication::read(paths, &plan.plan_sha256)?
            .is_some_and(|record| record.completed)
    {
        return Err(invalid());
    }
    for root in &bootstrap.roots {
        let held = root
            .capacity_import_bootstrap
            .as_ref()
            .ok_or_else(invalid)?;
        let target = desired
            .canisters
            .iter()
            .find(|entry| entry.name == root.root)
            .and_then(|entry| entry.principal.as_ref())
            .and_then(|id| Principal::from_text(id).ok())
            .ok_or_else(invalid)?;
        let expected = held.sources.iter().copied().collect::<BTreeSet<_>>();
        let matching = records
            .iter()
            .filter(|record| {
                record.plan.authority.root == target
                    && record.plan.authority.fleet == registry.authority.binding.fleet
                    && record.plan.authority.coordinator == registry.authority.binding.coordinator
                    && record.plan.authority.network_root_key_sha256
                        == source.network_root_key_sha256
                    && record.plan.authority.operator == source.operator
                    && record
                        .plan
                        .sources
                        .iter()
                        .map(|entry| entry.binding.canister_id)
                        .collect::<BTreeSet<_>>()
                        == expected
                    && record.operation.as_ref().is_some_and(|operation| {
                        operation.review.publication_kind
                            == CapacityImportPublicationKind::InitializeEstate
                    })
                    && publication::completed(record)
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(invalid());
        }
        for imported in &matching[0].plan.sources {
            if let Some(original) = source
                .held_sources
                .values()
                .find(|entry| entry.custody.canister == imported.binding.canister_id)
            {
                let actual = super::declarations::custody_from_binding(&imported.binding);
                if actual != original.custody || original.root != target {
                    return Err(invalid());
                }
                continue;
            }
            let original = source
                .sources
                .values()
                .find(|entry| entry.sample.binding.canister_id == imported.binding.canister_id)
                .ok_or_else(invalid)?;
            if !crate::fleet_ensure::ops::capacity_import::funding::bootstrap_source_matches(
                plan,
                &matching[0].plan,
                &original.sample,
                imported,
            ) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn read_import(
    path: &Path,
    records: &mut Vec<CapacityImportJournalRecord>,
) -> Result<(), InfrastructureBootstrapError> {
    if let Some(bytes) = read_optional_file_no_follow(path, MAX_BYTES)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?
    {
        let record = serde_json::from_slice(&bytes)?;
        journal::validate(&record)?;
        records.push(record);
    }
    Ok(())
}

/// A first workload plan has active registration but no completed workload inventory.
/// Prove that exception from the immutable setup archive before its first effect.
pub(in crate::fleet_ensure) fn verify_origin(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let archive = paths
        .plan
        .with_file_name("infrastructure-bootstrap-completed");
    let mut entries = std::fs::read_dir(&archive)?;
    let entry = entries.next().ok_or_else(invalid)??;
    if entries.next().is_some() || !entry.file_type()?.is_dir() {
        return Err(invalid());
    }
    let directory = entry.path();
    let original: FleetEnsurePlan = serde_json::from_slice(
        &read_file_no_follow(&directory.join("plan.json"), MAX_BYTES)
            .map_err(std::io::Error::from)?,
    )?;
    let journal = crate::fleet_ensure::ops::decode_journal(
        paths,
        &read_file_no_follow(&directory.join("journal.json"), MAX_BYTES)
            .map_err(std::io::Error::from)?,
    )?;
    let original_state: FleetEnsureStateRecord = serde_json::from_slice(
        &read_file_no_follow(&directory.join("state.json"), MAX_BYTES)
            .map_err(std::io::Error::from)?,
    )?;
    if original.scope != FleetEnsurePlanScope::InfrastructureBootstrap
        || plan.scope != FleetEnsurePlanScope::Full
        || original.plan_sha256 != crate::fleet_ensure::policy::expected_plan_sha256(&original)
        || entry.file_name() != original.plan_sha256.as_str()
        || original.operation_id != plan.operation_id
        || original.environment != plan.environment
        || original.fleet != plan.fleet
        || journal.completion != FleetEnsureCompletion::Converged
        || state.principals != original_state.principals
        || state.pending_principals != original_state.pending_principals
        || state.topology != original_state.topology
        || state.active_registry != original_state.active_registry
        || terminal::read_receipt(paths, &original, &journal, &original_state)?.is_none()
    {
        return Err(invalid());
    }
    verify_desired(
        &original,
        &original_state,
        plan.reviewed_desired
            .as_ref()
            .ok_or_else(invalid)?
            .desired(),
    )?;
    verify_imports(paths, &original, &original_state)
}

fn verify_desired(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    desired: &DesiredFleet,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let original = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let mut expected = original.clone();
    let bootstrap = expected.bootstrap.as_mut().ok_or_else(invalid)?;
    for root in &mut bootstrap.roots {
        root.capacity_import_bootstrap = None;
    }
    for canister in &mut expected.canisters {
        if canister.principal.is_none() {
            canister.principal = state.principals.get(&canister.name).cloned();
        }
    }
    if expected != *desired {
        return Err(invalid());
    }
    Ok(())
}
