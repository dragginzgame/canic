//! Persist current reset authority and import declarations from real observations.

pub(in crate::fleet_ensure) mod cancellation;
pub(in crate::fleet_ensure) mod terminal;

use crate::{
    durable_io::{read_optional_regular_bytes_bounded, write_bytes},
    fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest,
        model::{
            DesiredFleet, FleetEnsurePlan, ReviewedDesiredFleetRecord,
            capacity_import::{CapacityImportJournalRecord, survey::CapacityImportSampleRecord},
            clean_reinstall::CleanReinstallRecord,
        },
        ops::{
            EnsurePaths, EnsureStateError,
            capacity_import::{
                admission::{
                    CapacityImportDeclaration, CapacityImportDeclarations,
                    CapacityImportDispositionKind,
                },
                journal, publication,
            },
            read_current, write_current,
        },
    },
};
use candid::Principal;
use canic_core::cdk::utils::hash::hex_bytes;
use sha2_host::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

/// Read current reset authority after selection establishes its current owner.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
) -> Result<Option<CleanReinstallRecord>, EnsureStateError> {
    let record =
        read_current::<CleanReinstallRecord>(&paths.plan.with_file_name("clean-reinstall.json"))?;
    if record.as_ref().is_some_and(|record| {
        record.schema_version != 1
            || EnsurePaths::under(
                &paths.workspace,
                &record.desired.desired().environment,
                &record.desired.desired().fleet,
            ) != *paths
    }) {
        return Err(EnsureStateError::InvalidTerminalSource);
    }
    Ok(record)
}

/// Freeze operator paths alongside the selected build.
pub(in crate::fleet_ensure) fn bind(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    policy: &Path,
    seed: &Path,
) -> Result<CleanReinstallRecord, EnsureStateError> {
    let _lock = super::lock_fleet_file(paths)?;
    let record = CleanReinstallRecord {
        schema_version: 1,
        desired: ReviewedDesiredFleetRecord::capture(desired),
        policy: policy.into(),
        seed: seed.into(),
    };
    if let Some(retained) = read(paths)? {
        if retained != record {
            return Err(EnsureStateError::InvalidTerminalSource);
        }
    } else {
        write_current(&paths.plan.with_file_name("clean-reinstall.json"), &record)?;
    }
    Ok(record)
}

/// Import receipts are current because the prior operation was archived before setup.
pub(in crate::fleet_ensure) fn completed_roots(
    paths: &EnsurePaths,
) -> Result<BTreeSet<Principal>, EnsureStateError> {
    let mut files = vec![paths.plan.with_file_name("capacity-import.json")];
    let history = paths.plan.with_file_name("capacity-import-history");
    if history.exists() {
        for (index, entry) in std::fs::read_dir(&history)
            .map_err(|source| EnsureStateError::Io {
                path: history.clone(),
                source,
            })?
            .enumerate()
        {
            if index >= crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS {
                return Err(EnsureStateError::InvalidTerminalSource);
            }
            files.push(
                entry
                    .map_err(|source| EnsureStateError::Io {
                        path: history.clone(),
                        source,
                    })?
                    .path(),
            );
        }
    }
    let mut roots = BTreeSet::new();
    for path in files {
        if let Some(record) = read_current::<CapacityImportJournalRecord>(&path)? {
            journal::validate(&record).map_err(|source| {
                EnsureStateError::CapacityImportJournal {
                    path: path.clone(),
                    source: Box::new(source),
                }
            })?;
            if publication::completed(&record) {
                roots.insert(record.plan.authority.root);
            }
        }
    }
    Ok(roots)
}

/// Stable inspection identity retains the first successful management samples on retry.
pub(in crate::fleet_ensure) fn import_survey_digest(
    record: &CleanReinstallRecord,
    root: Principal,
) -> Result<[u8; 32], EnsureStateError> {
    let mut digest = Sha256::new();
    digest.update(b"canic.clean-reinstall.import-survey.v1\0");
    digest.update(
        serde_json::to_vec(&(record, root)).map_err(|_| EnsureStateError::InvalidTerminalSource)?,
    );
    Ok(digest.finalize().into())
}

/// Publish exact import inputs; the reset request explicitly disposes of selected App state.
pub(in crate::fleet_ensure) fn import_request(
    paths: &EnsurePaths,
    record: &CleanReinstallRecord,
    root: Principal,
    samples: &[CapacityImportSampleRecord],
    root_key: &[u8],
    maximum_call_debit_cycles: u128,
) -> Result<CapacityImportReviewRequest, EnsureStateError> {
    let desired = record.desired.desired();
    let declaration = CapacityImportDeclarations {
        schema_version: 1, operator: desired.operator.clone(), network_root_key_sha256: hex_bytes(Sha256::digest(root_key)),
        canisters: samples.iter().map(|sample| CapacityImportDeclaration {
            canister: sample.binding.canister_id.to_text(), subnet: sample.binding.subnet.to_string(),
            controllers: sample.binding.controllers.iter().map(Principal::to_text).collect(),
            module_sha256: sample.binding.module_sha256.map_or_else(|| "empty".into(), hex_bytes), canister_version: sample.binding.canister_version,
            disposition: CapacityImportDispositionKind::Retired, no_external_obligations: true, no_other_fleet_ownership: true,
            evidence: "Explicit selected Fleet clean reinstall; retain IDs and cycles, discard application and framework state".into(),
        }).collect(),
    };
    let declarations = paths
        .plan
        .with_file_name(format!("clean-reinstall-import-{root}.toml"));
    let bytes = toml::to_string(&declaration)
        .map_err(|_| EnsureStateError::InvalidTerminalSource)?
        .into_bytes();
    match read_optional_regular_bytes_bounded(&declarations, 256 * 1024).map_err(|_| {
        EnsureStateError::Unsafe {
            path: declarations.clone(),
        }
    })? {
        Some(existing) if existing == bytes => {}
        Some(_) => return Err(EnsureStateError::InvalidTerminalSource),
        None => write_bytes(&declarations, &bytes).map_err(|source| EnsureStateError::Io {
            path: declarations.clone(),
            source,
        })?,
    }
    let maximum_root_paid_calls =
        canic_core::control_plane_support::policy::pool_import::recommended_calls(samples.len())
            .ok_or(EnsureStateError::InvalidTerminalSource)?;
    let maximum_root_debit_cycles =
        canic_core::control_plane_support::policy::pool_import::required_debit(
            maximum_call_debit_cycles,
            maximum_root_paid_calls,
        )
        .ok_or(EnsureStateError::InvalidTerminalSource)?;
    Ok(CapacityImportReviewRequest {
        environment: desired.environment.clone(),
        fleet: desired.fleet.clone(),
        root: Some(root),
        canisters: samples
            .iter()
            .map(|sample| sample.binding.canister_id)
            .collect(),
        declarations,
        policy: record.policy.clone(),
        seed: record.seed.clone(),
        // These conservative ceilings are visible in the review before destructive import calls.
        maximum_source_debit_cycles: 100_000_000_000,
        maximum_root_debit_cycles,
        maximum_root_paid_calls,
    })
}

/// Bind fresh convergence to its completed setup so another reset gets distinct payment identity.
pub(in crate::fleet_ensure) fn convergence_digest(
    desired: &DesiredFleet,
    setup: &FleetEnsurePlan,
) -> Result<String, EnsureStateError> {
    let mut hash = Sha256::new();
    hash.update(b"canic.clean-reinstall.convergence.v1\0");
    hash.update(
        serde_json::to_vec(&(desired, &setup.plan_sha256))
            .map_err(|_| EnsureStateError::InvalidTerminalSource)?,
    );
    Ok(hex_bytes(hash.finalize()))
}
