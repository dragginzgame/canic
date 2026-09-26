//! Module: fleet_ensure::ops::reinstall::adoption::publication::archive
//!
//! Responsibility: retain and verify the immutable source and replacement closure.
//! Does not own: live authority, approval, active-file replacement or remote effects.
//! Boundary: recovery reads archived bytes independently of mutable source artifacts.

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{
        CompletedSourceInspectionView,
        model::{
            FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsureStateRecord,
            FleetTerminalSourceRecord,
            completed_handoff::{
                CompletedEstateDocumentsRecord, CompletedEstatePublicationReviewRecord,
            },
        },
        ops::{EnsurePaths, EnsureStateError, reinstall::adoption as storage},
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

pub(super) fn retain_target(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<CompletedEstateDocumentsRecord, EnsureStateError> {
    let save = |value: &[u8]| {
        let digest = sha256_hex(value);
        storage::retain(paths, &digest, value)?;
        Ok::<_, EnsureStateError>(digest)
    };
    Ok(CompletedEstateDocumentsRecord {
        plan_sha256: save(&super::serialize(plan, &paths.plan)?)?,
        journal_sha256: save(&super::serialize(journal, &paths.journal)?)?,
        state_sha256: save(&super::serialize(state, &paths.state)?)?,
    })
}

pub(super) fn retain_source(
    paths: &EnsurePaths,
    source: &FleetTerminalSourceRecord,
) -> Result<(), EnsureStateError> {
    for (path, digest) in [
        (&paths.plan, &source.plan_document_sha256),
        (&paths.journal, &source.journal_document_sha256),
        (&paths.state, &source.state_document_sha256),
    ] {
        let bytes = storage::exact_bytes(path, digest)?;
        storage::retain(paths, digest, &bytes)?;
    }
    storage::content::retain(
        paths,
        &storage::exact_bytes(&paths.plan, &source.plan_document_sha256)?,
    )?;
    for (label, digest) in &source.phase_document_sha256 {
        let path = paths
            .plan
            .with_file_name("phases")
            .join(format!("{label}.json"));
        let bytes = storage::exact_bytes(&path, digest)?;
        storage::retain(paths, digest, &bytes)?;
        storage::content::retain(paths, &bytes)?;
    }
    Ok(())
}

pub(super) fn retain_artifacts(
    paths: &EnsurePaths,
    source: &CompletedSourceInspectionView,
) -> Result<BTreeMap<String, String>, EnsureStateError> {
    let release = source.inventory.release_build_id;
    let directory = paths
        .workspace
        .join(".canic/release-builds")
        .join(release.to_string());
    let mut originals = [
        "plan.cbor",
        "current-release-set-manifest.json",
        "infrastructure-artifact-manifest.json",
        "application-artifact-union.json",
    ]
    .into_iter()
    .map(|name| directory.join(name))
    .collect::<Vec<_>>();
    originals.extend(
        source
            .source_protocols
            .values()
            .map(|protocol| protocol.candid_path().to_path_buf()),
    );
    let mut hashes = BTreeMap::new();
    for path in originals {
        let relative = path
            .strip_prefix(&paths.workspace)
            .map_err(|_| super::conflict())?;
        relative_path(relative)?;
        let bytes = read_regular_bytes(&path, 32 * 1024 * 1024)
            .map_err(|error| storage::io_error(&path, error))?;
        let digest = sha256_hex(&bytes);
        storage::retain(paths, &digest, &bytes)?;
        hashes.insert(
            relative.to_str().ok_or_else(super::conflict)?.to_string(),
            digest,
        );
    }
    Ok(hashes)
}

pub(super) fn verify_source_artifacts(
    paths: &EnsurePaths,
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<(), EnsureStateError> {
    for (relative, digest) in &review.source_artifacts {
        relative_path(Path::new(relative))?;
        storage::exact_bytes(&paths.workspace.join(relative), digest)?;
    }
    Ok(())
}

pub(super) fn verify(
    paths: &EnsurePaths,
    review: &CompletedEstatePublicationReviewRecord,
) -> Result<(), EnsureStateError> {
    let source = &review.source;
    for digest in [
        &source.plan_document_sha256,
        &source.journal_document_sha256,
        &source.state_document_sha256,
    ] {
        storage::exact_bytes(&storage::object_path(paths, digest), digest)?;
    }
    storage::content::verify(
        paths,
        &storage::exact_bytes(
            &storage::object_path(paths, &source.plan_document_sha256),
            &source.plan_document_sha256,
        )?,
    )?;
    for digest in source.phase_document_sha256.values() {
        let bytes = storage::exact_bytes(&storage::object_path(paths, digest), digest)?;
        storage::content::verify(paths, &bytes)?;
    }
    for (relative, digest) in &review.source_artifacts {
        relative_path(Path::new(relative))?;
        storage::exact_bytes(&storage::object_path(paths, digest), digest)?;
    }
    let target = &review.replacement;
    let plan: FleetEnsurePlan = archived(paths, &target.plan_sha256)?;
    let journal: FleetEnsureJournalRecord = archived(paths, &target.journal_sha256)?;
    let state: FleetEnsureStateRecord = archived(paths, &target.state_sha256)?;
    super::validate_target(&plan, &journal, &state)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(super::conflict)?
        .desired();
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(super::conflict)?;
    let observed_ids = review
        .custody
        .canisters
        .values()
        .map(|entry| entry.binding.principal.to_text())
        .collect::<std::collections::BTreeSet<_>>();
    let target_ids = state
        .principals
        .values()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if review.custody.operator.to_text() != desired.operator
        || review.custody.network != bootstrap.canonical_network_id
        || observed_ids.len() != review.custody.canisters.len()
        || observed_ids != target_ids
    {
        return Err(super::conflict());
    }
    if plan.operation_id != review.operation_id
        || plan.plan_sha256 != review.plan_sha256
        || plan.environment != review.environment
        || plan.fleet != review.fleet
    {
        return Err(super::conflict());
    }
    Ok(())
}

fn archived<T: serde::de::DeserializeOwned>(
    paths: &EnsurePaths,
    digest: &str,
) -> Result<T, EnsureStateError> {
    let bytes = storage::exact_bytes(&storage::object_path(paths, digest), digest)?;
    serde_json::from_slice(&bytes).map_err(|_| super::conflict())
}

fn relative_path(path: &Path) -> Result<(), EnsureStateError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(super::conflict());
    }
    Ok(())
}
