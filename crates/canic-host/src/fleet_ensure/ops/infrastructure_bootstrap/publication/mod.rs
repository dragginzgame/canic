//! Publish the reviewed estate seed after exact infrastructure completion, with local replay.

use crate::fleet_ensure::{
    generate::infrastructure_bootstrap::seed_projection,
    model::{
        DesiredFleet, FleetEnsurePlan,
        infrastructure_bootstrap::{
            InfrastructureBootstrapPublicationRecord, InfrastructureBootstrapRecord,
            InfrastructureBootstrapSeedRecord,
        },
    },
    ops::{
        self, EnsurePaths,
        infrastructure_bootstrap::{InfrastructureBootstrapError, seal_sources, terminal},
    },
    policy::expected_plan_sha256,
};

use candid::Principal;
use sha2_host::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

use ic_host_fs::durable::{read_optional_regular_bytes_bounded, read_regular_bytes, write_bytes};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_SEED_BYTES: usize = 1024 * 1024;

/// Bind the exact original seed before any initialization plan can be approved.
pub(in crate::fleet_ensure) fn bind(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
    seed: &Path,
) -> Result<InfrastructureBootstrapRecord, InfrastructureBootstrapError> {
    let workspace = paths.workspace.canonicalize()?;
    let path = if seed.is_absolute() {
        seed.to_owned()
    } else {
        workspace.join(seed)
    };
    let relative = path
        .strip_prefix(&workspace)
        .map_err(|_| invalid())?
        .to_str()
        .ok_or_else(invalid)?;
    let path = target(paths, relative)?;
    let bytes = read_regular_bytes(&path, MAX_SEED_BYTES)?;
    let original = String::from_utf8(bytes).map_err(|_| invalid())?;
    seed_projection(desired, &original, None).map_err(|_| invalid())?;
    let mut result = source.clone();
    result.estate_seed = Some(InfrastructureBootstrapSeedRecord {
        relative_path: relative.to_owned(),
        before_sha256: hash(original.as_bytes()),
        original,
    });
    seal_sources(result)
}

pub(in crate::fleet_ensure) fn validate_seed(
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
) -> Result<(), InfrastructureBootstrapError> {
    if let Some(seed) = &source.estate_seed {
        if seed.original.len() > MAX_SEED_BYTES
            || hash(seed.original.as_bytes()) != seed.before_sha256
        {
            return Err(invalid());
        }
        validate_path(&seed.relative_path)?;
        seed_projection(desired, &seed.original, None).map_err(|_| invalid())?;
    }
    Ok(())
}

/// Reserve the complete local publication envelope before paid setup effects.
pub(in crate::fleet_ensure) fn validate_capacity(
    plan: &FleetEnsurePlan,
) -> Result<(), InfrastructureBootstrapError> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let Some(seed) = plan
        .infrastructure_bootstrap
        .as_ref()
        .and_then(|source| source.estate_seed.as_ref())
    else {
        return Ok(());
    };
    let coordinator_name = &desired.bootstrap.as_ref().ok_or_else(invalid)?.coordinator;
    let coordinator = desired
        .canisters
        .iter()
        .find(|entry| entry.name == *coordinator_name)
        .and_then(|entry| entry.principal.as_ref())
        .map(|id| Principal::from_text(id).map_err(|_| invalid()))
        .transpose()?
        .unwrap_or_else(|| Principal::from_slice(&[255; 29]));
    let replacement =
        seed_projection(desired, &seed.original, Some(coordinator)).map_err(|_| invalid())?;
    if replacement.len() > MAX_SEED_BYTES {
        return Err(invalid());
    }
    let largest = InfrastructureBootstrapPublicationRecord {
        schema_version: 1,
        plan: plan.clone(),
        replacement,
        after_sha256: [255; 32],
        terminal_receipt_sha256: [255; 32],
        coordinator,
        completed: false,
    };
    if serde_json::to_vec_pretty(&largest)?.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(())
}

fn path(paths: &EnsurePaths, digest: &str) -> Result<PathBuf, InfrastructureBootstrapError> {
    if !ops::is_sha256(digest) {
        return Err(invalid());
    }
    Ok(paths
        .plan
        .with_file_name("infrastructure-bootstrap-publications")
        .join(format!("{digest}.json")))
}

/// Completed replay validates immutable local evidence before any signer or network resolution.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    digest: &str,
) -> Result<Option<InfrastructureBootstrapPublicationRecord>, InfrastructureBootstrapError> {
    let Some(bytes) = read_optional_regular_bytes_bounded(&path(paths, digest)?, MAX_BYTES)
        .map_err(|_| invalid())?
    else {
        return Ok(None);
    };
    let record: InfrastructureBootstrapPublicationRecord = serde_json::from_slice(&bytes)?;
    let desired = record
        .plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let source = record
        .plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or_else(invalid)?;
    let seed = source.estate_seed.as_ref().ok_or_else(invalid)?;
    validate_seed(desired, source)?;
    let receipt_bytes = read_regular_bytes(&terminal::path(paths, &record.plan), MAX_BYTES)?;
    let receipt: crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapTerminalRecord = serde_json::from_slice(&receipt_bytes)?;
    let coordinator = &desired.bootstrap.as_ref().ok_or_else(invalid)?.coordinator;
    if record.schema_version != 1
        || record.plan.plan_sha256 != digest
        || record.plan.plan_sha256 != expected_plan_sha256(&record.plan)
        || hash(record.replacement.as_bytes()) != record.after_sha256
        || seed_projection(desired, &seed.original, Some(record.coordinator))
            .map_err(|_| invalid())?
            != record.replacement
        || hash(&receipt_bytes) != record.terminal_receipt_sha256
        || receipt.plan_sha256 != digest
        || receipt
            .canisters
            .get(coordinator)
            .is_none_or(|sample| sample.binding.canister_id != record.coordinator)
    {
        return Err(invalid());
    }
    Ok(Some(record))
}

/// Retain single-file intent, replace exact prior bytes, then retain completion under the Fleet lock.
pub(in crate::fleet_ensure) fn publish(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
) -> Result<InfrastructureBootstrapPublicationRecord, InfrastructureBootstrapError> {
    let _lock = ops::lock_operation(paths)?;
    if let Some(record) = read(paths, &plan.plan_sha256)? {
        if record.plan != *plan {
            return Err(invalid());
        }
        return finish(paths, record);
    }
    let state = ops::read_state(paths, &plan.fleet)?;
    let journal = ops::read_journal(paths)?.ok_or_else(invalid)?;
    terminal::read_receipt(paths, plan, &journal, &state)?.ok_or_else(invalid)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(invalid)?;
    let seed = plan
        .infrastructure_bootstrap
        .as_ref()
        .and_then(|source| source.estate_seed.as_ref())
        .ok_or_else(invalid)?;
    let coordinator = state
        .principals
        .get(&bootstrap.coordinator)
        .and_then(|id| Principal::from_text(id).ok())
        .ok_or_else(invalid)?;
    let replacement =
        seed_projection(desired, &seed.original, Some(coordinator)).map_err(|_| invalid())?;
    let record = InfrastructureBootstrapPublicationRecord {
        schema_version: 1,
        plan: plan.clone(),
        after_sha256: hash(replacement.as_bytes()),
        replacement,
        terminal_receipt_sha256: hash(&read_regular_bytes(
            &terminal::path(paths, plan),
            MAX_BYTES,
        )?),
        coordinator,
        completed: false,
    };
    save(paths, &record)?;
    finish(paths, record)
}

fn finish(
    paths: &EnsurePaths,
    mut record: InfrastructureBootstrapPublicationRecord,
) -> Result<InfrastructureBootstrapPublicationRecord, InfrastructureBootstrapError> {
    if record.completed {
        return Ok(record);
    }
    let seed = record
        .plan
        .infrastructure_bootstrap
        .as_ref()
        .and_then(|source| source.estate_seed.as_ref())
        .ok_or_else(invalid)?;
    let destination = target(paths, &seed.relative_path)?;
    let bytes = read_regular_bytes(&destination, MAX_SEED_BYTES)?;
    if hash(&bytes) != record.after_sha256 {
        if bytes != seed.original.as_bytes() {
            return Err(invalid());
        }
        write_bytes(&destination, record.replacement.as_bytes())?;
    }
    record.completed = true;
    save(paths, &record)?;
    Ok(record)
}

fn save(
    paths: &EnsurePaths,
    record: &InfrastructureBootstrapPublicationRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let bytes = serde_json::to_vec_pretty(record)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(write_bytes(
        &path(paths, &record.plan.plan_sha256)?,
        &bytes,
    )?)
}

fn validate_path(relative: &str) -> Result<(), InfrastructureBootstrapError> {
    if relative.is_empty()
        || !Path::new(relative)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(invalid());
    }
    Ok(())
}
fn target(paths: &EnsurePaths, relative: &str) -> Result<PathBuf, InfrastructureBootstrapError> {
    validate_path(relative)?;
    let path = paths.workspace.canonicalize()?.join(relative);
    if path.canonicalize()? != path {
        return Err(invalid());
    }
    Ok(path)
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
const fn invalid() -> InfrastructureBootstrapError {
    InfrastructureBootstrapError::Integrity
}

/// Retain the final source digest in every Root's exact allocation hold.
pub(in crate::fleet_ensure) fn bind_holds(
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
) -> Result<DesiredFleet, InfrastructureBootstrapError> {
    let mut result = desired.clone();
    let bootstrap = result.bootstrap.as_mut().ok_or_else(invalid)?;
    for root in &mut bootstrap.roots {
        let mut sources = root
            .canister_pool_imports
            .iter()
            .map(|name| {
                let configured = desired
                    .canisters
                    .iter()
                    .find(|entry| entry.name == *name)
                    .ok_or_else(invalid)?;
                Principal::from_text(configured.principal.as_ref().ok_or_else(invalid)?)
                    .map_err(|_| invalid())
            })
            .collect::<Result<Vec<_>, _>>()?;
        sources.sort_unstable();
        if sources.is_empty()
            || sources.len() > canic_core::ids::MAX_FLEET_CAPACITY_IMPORT_SOURCES
            || sources.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(invalid());
        }
        root.capacity_import_bootstrap = Some(
            crate::fleet_ensure::model::capacity_import::CapacityImportBootstrapRecord {
                review_sha256: source.source_sha256,
                operator: source.operator,
                sources,
            },
        );
    }
    Ok(result)
}
